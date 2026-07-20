use std::fmt;

use crate::security::{
    AuthorizedBinanceQuery, CredentialHandle, CredentialSigningError, CredentialVault, Planning,
    SignBinanceQuery, VaultError,
};

use super::{BinanceRequestTarget, ClockError, RequestWeight, ServerClock};

const RECV_WINDOW_MILLIS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinanceReadEndpoint {
    ServerTime,
    ApiRestrictions,
    Account,
    AllCoins,
    SystemStatus,
    WithdrawalQuota,
    ExchangeInfo,
    TickerPrice,
    SavedWithdrawalAddresses,
}

impl BinanceReadEndpoint {
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::ServerTime => "/api/v3/time",
            Self::ApiRestrictions => "/sapi/v1/account/apiRestrictions",
            Self::Account => "/api/v3/account",
            Self::AllCoins => "/sapi/v1/capital/config/getall",
            Self::SystemStatus => "/sapi/v1/system/status",
            Self::WithdrawalQuota => "/sapi/v1/capital/withdraw/quota",
            Self::ExchangeInfo => "/api/v3/exchangeInfo",
            Self::TickerPrice => "/api/v3/ticker/price",
            Self::SavedWithdrawalAddresses => "/sapi/v1/capital/withdraw/address/list",
        }
    }

    #[must_use]
    pub const fn requires_signature(self) -> bool {
        !matches!(
            self,
            Self::ServerTime | Self::SystemStatus | Self::ExchangeInfo | Self::TickerPrice
        )
    }

    pub(super) const fn weight(self) -> RequestWeight {
        // Policy snapshot reviewed 2026-07-14. Task 5 must re-check the
        // provider's current endpoint catalog before enabling live sync;
        // response headers remain authoritative after every request.
        match self {
            Self::ServerTime | Self::SystemStatus | Self::ApiRestrictions => {
                RequestWeight::new(1, 1)
            }
            Self::Account | Self::ExchangeInfo => RequestWeight::new(20, 20),
            Self::AllCoins | Self::WithdrawalQuota | Self::SavedWithdrawalAddresses => {
                RequestWeight::new(10, 10)
            }
            Self::TickerPrice => RequestWeight::new(4, 4),
        }
    }
}

pub(crate) struct SignedQuery {
    endpoint: BinanceReadEndpoint,
    parameter_names: Vec<String>,
    authorization: AuthorizedBinanceQuery,
}

impl SignedQuery {
    /// Canonicalizes parameters, applies bounded server time, and signs through
    /// the vault-owned credential operation.
    ///
    /// # Errors
    /// Returns an error for unsigned endpoints, reserved parameters, unsafe
    /// clock state, vault failure, or credential-signing failure.
    pub(crate) fn new<Vault, Parameters, Key, Value>(
        vault: &Vault,
        credential: &CredentialHandle<Planning>,
        endpoint: BinanceReadEndpoint,
        parameters: Parameters,
        clock: &ServerClock,
        local_timestamp_millis: u64,
    ) -> Result<Self, SignedQueryError>
    where
        Vault: CredentialVault,
        Parameters: IntoIterator<Item = (Key, Value)>,
        Key: AsRef<str>,
        Value: AsRef<str>,
    {
        if !endpoint.requires_signature() {
            return Err(SignedQueryError::UnsignedEndpoint);
        }

        let mut encoded = Vec::new();
        for (key, value) in parameters {
            let key = key.as_ref();
            if matches!(key, "recvWindow" | "timestamp" | "signature") {
                return Err(SignedQueryError::ReservedParameter);
            }
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            serializer.append_pair(key, value.as_ref());
            encoded.push((serializer.finish(), key.to_owned()));
        }
        encoded.sort_unstable_by(|left, right| left.0.cmp(&right.0));

        let corrected_timestamp = clock.corrected_timestamp(local_timestamp_millis)?;
        let mut canonical_query = encoded
            .iter()
            .map(|(pair, _)| pair.as_str())
            .collect::<Vec<_>>()
            .join("&");
        if !canonical_query.is_empty() {
            canonical_query.push('&');
        }
        canonical_query.push_str("recvWindow=");
        canonical_query.push_str(&RECV_WINDOW_MILLIS.to_string());
        canonical_query.push_str("&timestamp=");
        canonical_query.push_str(&corrected_timestamp.to_string());

        let authorization = vault
            .with_credential(credential, SignBinanceQuery::new(canonical_query))
            .map_err(SignedQueryError::Vault)??;
        let mut parameter_names = encoded
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
        parameter_names.extend(
            ["recvWindow", "timestamp", "signature"]
                .into_iter()
                .map(str::to_owned),
        );

        Ok(Self {
            endpoint,
            parameter_names,
            authorization,
        })
    }

    #[must_use]
    pub(crate) fn endpoint(&self) -> BinanceReadEndpoint {
        self.endpoint
    }

    #[must_use]
    #[cfg(test)]
    fn parameter_names(&self) -> &[String] {
        &self.parameter_names
    }

    pub(crate) fn into_request(self, target: BinanceRequestTarget) -> reqwest::RequestBuilder {
        self.authorization.apply(target)
    }
}

impl fmt::Debug for SignedQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignedQuery")
            .field("method", &"GET")
            .field("path", &self.endpoint.path())
            .field("parameter_names", &self.parameter_names)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SignedQueryError {
    #[error("signed queries require a signed read-only Binance endpoint")]
    UnsignedEndpoint,
    #[error("caller parameters cannot override Binance signing fields")]
    ReservedParameter,
    #[error(transparent)]
    Clock(#[from] ClockError),
    #[error(transparent)]
    Vault(VaultError),
    #[error(transparent)]
    Credential(#[from] CredentialSigningError),
}

#[cfg(test)]
mod tests {
    use crate::security::{CredentialVault, MemoryCredentialVault, SecretCredential};

    use super::*;

    #[test]
    fn signed_query_debug_exposes_only_fixed_request_shape() {
        let vault = MemoryCredentialVault::default();
        let handle = vault
            .store(SecretCredential::new_binance_hmac(
                "fixture-api-key",
                "fixture-secret",
            ))
            .expect("fixture credential is stored");
        let mut clock = ServerClock::new(50);
        clock.observe(1_000, 1_500, 1_000).expect("clock syncs");
        let query = SignedQuery::new(
            &vault,
            &handle,
            BinanceReadEndpoint::Account,
            [("symbol", "ETH BTC"), ("note", "a+b")],
            &clock,
            2_000,
        )
        .expect("query signs");
        assert_eq!(
            query.parameter_names(),
            ["note", "symbol", "recvWindow", "timestamp", "signature"].map(str::to_owned)
        );
        let rendered = format!("{query:?}");
        for forbidden in ["fixture-api-key", "fixture-secret", "ETH BTC", "signature="] {
            assert!(!rendered.contains(forbidden));
        }
    }
}
