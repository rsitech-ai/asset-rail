use std::{fmt, sync::Mutex, time::Duration};

use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use url::Url;

#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
use super::DeterministicRateClock;
use super::{
    BinanceReadEndpoint, BudgetError, RateBudget, RefreshPriority, RequestWeight, SignedQuery,
};

const PRODUCTION_ORIGIN: &str = "https://api.binance.com";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RESPONSE_BODY_LIMIT_BYTES: usize = 1_048_576;
const CATALOG_RESPONSE_BODY_LIMIT_BYTES: usize = 8_388_608;

pub struct BinanceTransport {
    client: Client,
    origin: Url,
    origin_kind: OriginKind,
    timeout: Duration,
    budget: Mutex<RateBudget>,
}

/// Unforgeable proof that an authenticated Rust application service approved
/// use of capacity reserved for an explicit operator refresh.
pub struct OperatorRefreshPermit {
    _private: (),
}

pub(crate) struct BinanceRequestTarget {
    client: Client,
    url: Url,
}

impl BinanceRequestTarget {
    fn new(client: Client, url: Url) -> Self {
        Self { client, url }
    }

    pub(crate) fn authorize(
        self,
        api_key: HeaderValue,
        signed_query: SecretString,
    ) -> reqwest::RequestBuilder {
        let mut url = self.url;
        url.set_query(Some(signed_query.expose_secret()));
        drop(signed_query);
        self.client.get(url).header("X-MBX-APIKEY", api_key)
    }
}

#[derive(Clone, Copy, Debug)]
enum OriginKind {
    Production,
    #[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
    LoopbackTest,
}

impl BinanceTransport {
    /// Creates the production transport with the fixed Binance origin.
    ///
    /// # Errors
    /// Returns an error only if the pinned production origin or TLS client
    /// cannot be initialized.
    pub fn new(mut budget: RateBudget) -> Result<Self, TransportError> {
        budget.install_production_clock();
        Self::build(
            Url::parse(PRODUCTION_ORIGIN).map_err(|_| TransportError::InvalidOrigin)?,
            OriginKind::Production,
            DEFAULT_TIMEOUT,
            budget,
        )
    }

    #[must_use]
    pub const fn production_origin() -> &'static str {
        PRODUCTION_ORIGIN
    }

    #[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
    /// Creates a transport for an injected loopback-only test server.
    ///
    /// This constructor is absent from optimized release builds.
    ///
    /// # Errors
    /// Returns an error when client initialization fails.
    pub fn with_test_transport(
        test_transport: LoopbackTestTransport,
        timeout: Duration,
        mut budget: RateBudget,
    ) -> Result<Self, TransportError> {
        budget.install_test_clock(test_transport.clock.clone());
        Self::build(
            test_transport.origin,
            OriginKind::LoopbackTest,
            timeout,
            budget,
        )
    }

    fn build(
        origin: Url,
        origin_kind: OriginKind,
        timeout: Duration,
        budget: RateBudget,
    ) -> Result<Self, TransportError> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|_| TransportError::ClientInitialization)?;
        Ok(Self {
            client,
            origin,
            origin_kind,
            timeout,
            budget: Mutex::new(budget),
        })
    }

    /// Sends a fixed-endpoint unsigned Binance GET request.
    ///
    /// # Errors
    /// Returns a typed error for endpoint misuse, rate policy, network,
    /// timeout, provider status, or response-state failure.
    pub async fn get_public<Parameters, Key, Value>(
        &self,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
    ) -> Result<HttpResponse, TransportError>
    where
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        self.get_public_with_priority(endpoint, parameters, RefreshPriority::Background)
            .await
    }

    /// Sends an explicit operator-initiated public refresh using reserved capacity.
    ///
    /// # Errors
    /// Returns the same typed failures as [`Self::get_public`].
    pub async fn get_public_for_operator<Parameters, Key, Value>(
        &self,
        _permit: &OperatorRefreshPermit,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
    ) -> Result<HttpResponse, TransportError>
    where
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        self.get_public_with_priority(endpoint, parameters, RefreshPriority::Operator)
            .await
    }

    async fn get_public_with_priority<Parameters, Key, Value>(
        &self,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
        priority: RefreshPriority,
    ) -> Result<HttpResponse, TransportError>
    where
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        if endpoint.requires_signature() {
            return Err(TransportError::SignedEndpointRequiresAuthorization);
        }
        self.reserve(endpoint.weight(), priority)?;

        let mut url = self.endpoint_url(endpoint)?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in parameters {
                query.append_pair(key.as_ref(), value.as_ref());
            }
        }
        self.execute(self.client.get(url), response_body_limit(endpoint))
            .await
    }

    /// Sends a query authorized by the sealed credential-vault operation.
    ///
    /// # Errors
    /// Returns a typed error for endpoint misuse, rate policy, network,
    /// timeout, provider status, or response-state failure.
    pub async fn get_signed<Vault, Parameters, Key, Value>(
        &self,
        vault: &Vault,
        credential: &crate::security::CredentialHandle<crate::security::Planning>,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
        clock: &super::ServerClock,
        local_timestamp_millis: u64,
    ) -> Result<HttpResponse, TransportError>
    where
        Vault: crate::security::CredentialVault,
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        self.get_signed_with_priority(
            vault,
            credential,
            endpoint,
            parameters,
            clock,
            local_timestamp_millis,
            RefreshPriority::Background,
        )
        .await
    }

    /// Sends an explicit operator-initiated signed refresh using reserved capacity.
    ///
    /// # Errors
    /// Returns the same typed failures as [`Self::get_signed`].
    #[allow(clippy::too_many_arguments)]
    pub async fn get_signed_for_operator<Vault, Parameters, Key, Value>(
        &self,
        _permit: &OperatorRefreshPermit,
        vault: &Vault,
        credential: &crate::security::CredentialHandle<crate::security::Planning>,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
        clock: &super::ServerClock,
        local_timestamp_millis: u64,
    ) -> Result<HttpResponse, TransportError>
    where
        Vault: crate::security::CredentialVault,
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        self.get_signed_with_priority(
            vault,
            credential,
            endpoint,
            parameters,
            clock,
            local_timestamp_millis,
            RefreshPriority::Operator,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn get_signed_with_priority<Vault, Parameters, Key, Value>(
        &self,
        vault: &Vault,
        credential: &crate::security::CredentialHandle<crate::security::Planning>,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
        clock: &super::ServerClock,
        local_timestamp_millis: u64,
        priority: RefreshPriority,
    ) -> Result<HttpResponse, TransportError>
    where
        Vault: crate::security::CredentialVault,
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        let query = SignedQuery::new(
            vault,
            credential,
            endpoint,
            parameters,
            clock,
            local_timestamp_millis,
        )?;
        self.reserve(endpoint.weight(), priority)?;
        let endpoint = query.endpoint();
        let url = self.endpoint_url(endpoint)?;
        let request = query.into_request(BinanceRequestTarget::new(self.client.clone(), url));
        self.execute(request, response_body_limit(endpoint)).await
    }

    /// Returns a non-secret point-in-time copy of current rate state.
    ///
    /// # Errors
    /// Returns [`TransportError::StateUnavailable`] if the budget lock was poisoned.
    pub fn budget_snapshot(&self) -> Result<RateBudget, TransportError> {
        self.budget
            .lock()
            .map(|budget| budget.clone())
            .map_err(|_| TransportError::StateUnavailable)
    }

    fn endpoint_url(&self, endpoint: BinanceReadEndpoint) -> Result<Url, TransportError> {
        self.origin
            .join(endpoint.path())
            .map_err(|_| TransportError::InvalidOrigin)
    }

    fn reserve(
        &self,
        weight: RequestWeight,
        priority: RefreshPriority,
    ) -> Result<(), TransportError> {
        self.budget
            .lock()
            .map_err(|_| TransportError::StateUnavailable)?
            .reserve(weight, priority)
            .map_err(TransportError::Budget)
    }

    async fn execute(
        &self,
        request: reqwest::RequestBuilder,
        maximum_body_bytes: usize,
    ) -> Result<HttpResponse, TransportError> {
        let mut response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                TransportError::Timeout
            } else {
                TransportError::Network
            }
        })?;
        let status = response.status();
        let headers = response.headers().clone();
        self.budget
            .lock()
            .map_err(|_| TransportError::StateUnavailable)?
            .observe_response(status, &headers)
            .map_err(TransportError::Budget)?;

        if matches!(status.as_u16(), 418 | 429) {
            return Err(TransportError::RateLimited {
                status: status.as_u16(),
            });
        }
        if !status.is_success() {
            return Err(TransportError::HttpStatus {
                status: status.as_u16(),
            });
        }
        if response.content_length().is_some_and(|length| {
            usize::try_from(length).map_or(true, |length| length > maximum_body_bytes)
        }) {
            return Err(TransportError::ResponseTooLarge);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            if error.is_timeout() {
                TransportError::Timeout
            } else {
                TransportError::Network
            }
        })? {
            let next_length = body
                .len()
                .checked_add(chunk.len())
                .ok_or(TransportError::ResponseTooLarge)?;
            if next_length > maximum_body_bytes {
                return Err(TransportError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

const fn response_body_limit(endpoint: BinanceReadEndpoint) -> usize {
    match endpoint {
        BinanceReadEndpoint::AllCoins | BinanceReadEndpoint::ExchangeInfo => {
            CATALOG_RESPONSE_BODY_LIMIT_BYTES
        }
        _ => DEFAULT_RESPONSE_BODY_LIMIT_BYTES,
    }
}

#[cfg(all(test, feature = "test-transport"))]
mod tests {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    #[tokio::test]
    async fn crate_internal_operator_refresh_uses_reserved_capacity() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/time"))
            .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
            .expect(2)
            .mount(&server)
            .await;
        let transport = BinanceTransport::with_test_transport(
            LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
            Duration::from_secs(1),
            RateBudget::new(2, 2, 1),
        )
        .expect("test transport is constructed");

        transport
            .get_public(
                BinanceReadEndpoint::ServerTime,
                std::iter::empty::<(&str, &str)>(),
            )
            .await
            .expect("background refresh uses non-reserved capacity");
        assert!(matches!(
            transport
                .get_public(
                    BinanceReadEndpoint::ServerTime,
                    std::iter::empty::<(&str, &str)>(),
                )
                .await,
            Err(TransportError::Budget(
                BudgetError::ReservedOperatorCapacity { .. }
            ))
        ));
        transport
            .get_public_for_operator(
                &OperatorRefreshPermit { _private: () },
                BinanceReadEndpoint::ServerTime,
                std::iter::empty::<(&str, &str)>(),
            )
            .await
            .expect("only the crate-internal operator path may use reserved capacity");
    }
}

impl fmt::Debug for BinanceTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BinanceTransport")
            .field("origin", &self.origin_kind)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
#[derive(Clone)]
pub struct LoopbackTestTransport {
    origin: Url,
    clock: DeterministicRateClock,
}

#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
impl LoopbackTestTransport {
    /// Validates a loopback-only HTTP(S) origin for an injected test server.
    ///
    /// # Errors
    /// Returns an error for non-loopback hosts, credentials, paths, queries,
    /// fragments, or non-HTTP schemes.
    pub fn new(origin: &str) -> Result<Self, TransportError> {
        let origin = Url::parse(origin).map_err(|_| TransportError::InvalidTestOrigin)?;
        let host_is_loopback = match origin.host_str() {
            Some("localhost") => true,
            Some(host) => host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback()),
            None => false,
        };
        let path_is_origin = matches!(origin.path(), "" | "/");
        if !matches!(origin.scheme(), "http" | "https")
            || !host_is_loopback
            || !origin.username().is_empty()
            || origin.password().is_some()
            || !path_is_origin
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(TransportError::InvalidTestOrigin);
        }
        Ok(Self {
            origin,
            clock: DeterministicRateClock::new(0),
        })
    }

    /// Sets the deterministic monotonic clock used only by loopback tests.
    pub fn set_now_millis(&self, now_millis: u64) {
        self.clock.set_now_millis(now_millis);
    }
}

pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl HttpResponse {
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Deserializes the bounded response body without including it in errors.
    ///
    /// # Errors
    /// Returns a redacted parse error when JSON is malformed or incompatible.
    pub fn json<Value: DeserializeOwned>(&self) -> Result<Value, TransportError> {
        serde_json::from_slice(&self.body).map_err(|_| TransportError::MalformedJson)
    }
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpResponse")
            .field("status", &self.status.as_u16())
            .field("body_length", &self.body.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TransportError {
    #[error("the fixed Binance origin is invalid")]
    InvalidOrigin,
    #[error("the injected test origin must be a credential-free loopback HTTP(S) origin")]
    InvalidTestOrigin,
    #[error("the Binance HTTP client could not be initialized")]
    ClientInitialization,
    #[error("signed Binance endpoints require sealed authorization")]
    SignedEndpointRequiresAuthorization,
    #[error(transparent)]
    Budget(BudgetError),
    #[error("Binance transport state is unavailable")]
    StateUnavailable,
    #[error("Binance request timed out")]
    Timeout,
    #[error("Binance network request failed; details were redacted")]
    Network,
    #[error("Binance rate limited the request with HTTP {status}")]
    RateLimited { status: u16 },
    #[error("Binance returned HTTP {status}")]
    HttpStatus { status: u16 },
    #[error("Binance response JSON was malformed; body was redacted")]
    MalformedJson,
    #[error("Binance response exceeded the endpoint body-size limit")]
    ResponseTooLarge,
    #[error(transparent)]
    Signing(#[from] super::SignedQueryError),
}
