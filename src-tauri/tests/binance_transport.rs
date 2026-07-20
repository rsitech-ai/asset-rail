#![cfg(all(feature = "test-transport", debug_assertions))]

use assetrail_lib::connectors::ConnectorId;
use assetrail_lib::connectors::binance::{
    BinanceReadEndpoint, BinanceTransport, LoopbackTestTransport, RateBudget, ServerClock,
    TransportError, WeightScope,
};
use assetrail_lib::security::{CredentialVault, MemoryCredentialVault, SecretCredential};
use reqwest::StatusCode;
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn transport_keeps_production_fixed_and_allows_only_injected_loopback_tests() {
    assert_eq!(
        BinanceTransport::production_origin(),
        "https://api.binance.com"
    );
    assert!(LoopbackTestTransport::new("https://api.binance.com").is_err());
    assert!(LoopbackTestTransport::new("http://127.0.0.1:8080/extra").is_err());

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/time"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("X-MBX-USED-WEIGHT-1M", "17")
                .insert_header("X-SAPI-USED-UID-WEIGHT-1M", "23")
                .set_body_raw("fixture-sensitive-body", "application/json"),
        )
        .mount(&server)
        .await;

    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");
    let response = transport
        .get_public(
            BinanceReadEndpoint::ServerTime,
            std::iter::empty::<(&str, &str)>(),
        )
        .await
        .expect("public request reaches only Wiremock");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.body(), b"fixture-sensitive-body");
    assert!(!format!("{response:?}").contains("fixture-sensitive-body"));
    let budget = transport
        .budget_snapshot()
        .expect("non-secret budget state is available");
    assert_eq!(budget.used(WeightScope::Ip), 17);
    assert_eq!(budget.used(WeightScope::Uid), 23);
}

#[tokio::test]
async fn transport_enforces_its_request_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/time"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(100)))
        .mount(&server)
        .await;
    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
        Duration::from_millis(20),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");

    let error = transport
        .get_public(
            BinanceReadEndpoint::ServerTime,
            std::iter::empty::<(&str, &str)>(),
        )
        .await
        .expect_err("delayed Wiremock response must time out");

    assert_eq!(error, TransportError::Timeout);
}

#[tokio::test]
async fn transport_rejects_a_fast_oversized_provider_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/time"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1_048_577]))
        .mount(&server)
        .await;
    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");

    let result = transport
        .get_public(
            BinanceReadEndpoint::ServerTime,
            std::iter::empty::<(&str, &str)>(),
        )
        .await;

    assert!(matches!(result, Err(TransportError::ResponseTooLarge)));
}

#[tokio::test]
async fn signed_transport_sends_the_exact_canonical_query_without_debug_exposure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/account"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&server)
        .await;
    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(SecretCredential::new_binance_hmac(
            "fixture-api-key",
            "fixture-secret",
        ))
        .expect("fixture credential is stored");
    let mut clock = ServerClock::new(10);
    clock
        .observe(1_000, 1_500, 1_000)
        .expect("clock fixture is synchronized");
    transport
        .get_signed(
            &vault,
            &handle,
            BinanceReadEndpoint::Account,
            [("symbol", "ETH BTC"), ("note", "a+b")],
            &clock,
            2_000,
        )
        .await
        .expect("signed request reaches only Wiremock");

    let requests = server
        .received_requests()
        .await
        .expect("Wiremock records requests");
    assert_eq!(requests.len(), 1);
    let canonical = "note=a%2Bb&symbol=ETH+BTC&recvWindow=5000&timestamp=2500";
    let query = requests[0].url.query().expect("signed query is present");
    assert!(query.starts_with(&format!("{canonical}&signature=")));
    let signature = query.rsplit_once("signature=").expect("signature exists").1;
    assert_eq!(signature.len(), 64);
    assert_eq!(
        requests[0]
            .headers
            .get("x-mbx-apikey")
            .expect("signed request has the API-key header")
            .to_str()
            .expect("fixture header is ASCII"),
        "fixture-api-key"
    );
    let rendered = format!("{transport:?}");
    assert!(!rendered.contains("fixture-api-key"));
    assert!(!rendered.contains("fixture-secret"));
    assert!(!rendered.contains(signature));
}

#[tokio::test]
async fn signed_transport_rejects_a_credential_scoped_to_another_connector() {
    let server = MockServer::start().await;
    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only"),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(SecretCredential::new_hmac(
            ConnectorId::parse("kraken-spot").expect("fixture connector is valid"),
            "other-api-key",
            "other-secret",
        ))
        .expect("fixture credential is stored");
    let mut clock = ServerClock::new(10);
    clock
        .observe(1_000, 1_000, 1_000)
        .expect("clock fixture is synchronized");

    let result = transport
        .get_signed(
            &vault,
            &handle,
            BinanceReadEndpoint::Account,
            std::iter::empty::<(&str, &str)>(),
            &clock,
            2_000,
        )
        .await;

    assert!(matches!(
        result,
        Err(TransportError::Signing(
            assetrail_lib::connectors::binance::SignedQueryError::Credential(
                assetrail_lib::security::CredentialSigningError::ScopeMismatch
            )
        ))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("Wiremock records requests")
            .is_empty()
    );
}

#[tokio::test]
async fn signed_transport_never_follows_a_cross_origin_redirect() {
    let redirect_target = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/capture"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&redirect_target)
        .await;

    let source = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/account"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", format!("{}/capture", redirect_target.uri())),
        )
        .mount(&source)
        .await;

    let transport = BinanceTransport::with_test_transport(
        LoopbackTestTransport::new(&source.uri()).expect("Wiremock is loopback-only"),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(SecretCredential::new_binance_hmac(
            "redirect-api-key",
            "redirect-secret",
        ))
        .expect("fixture credential is stored");
    let mut clock = ServerClock::new(10);
    clock
        .observe(1_000, 1_000, 1_000)
        .expect("clock fixture is synchronized");
    let result = transport
        .get_signed(
            &vault,
            &handle,
            BinanceReadEndpoint::Account,
            std::iter::empty::<(&str, &str)>(),
            &clock,
            2_000,
        )
        .await;

    assert!(matches!(
        result,
        Err(TransportError::HttpStatus { status: 302 })
    ));
    assert!(
        redirect_target
            .received_requests()
            .await
            .expect("redirect target records requests")
            .is_empty(),
        "the redirect target must receive neither a request nor X-MBX-APIKEY"
    );
}

#[tokio::test]
async fn transport_honors_retry_after_before_allowing_another_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/time"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "3"))
        .expect(1)
        .mount(&server)
        .await;
    let test_transport =
        LoopbackTestTransport::new(&server.uri()).expect("Wiremock is loopback-only");
    test_transport.set_now_millis(1_000);
    let transport = BinanceTransport::with_test_transport(
        test_transport.clone(),
        Duration::from_secs(1),
        RateBudget::new(100, 100, 10),
    )
    .expect("test transport is constructed");

    let first = transport
        .get_public(
            BinanceReadEndpoint::ServerTime,
            std::iter::empty::<(&str, &str)>(),
        )
        .await;
    assert!(matches!(
        first,
        Err(TransportError::RateLimited { status: 429 })
    ));

    test_transport.set_now_millis(3_999);
    let blocked = transport
        .get_public(
            BinanceReadEndpoint::ServerTime,
            std::iter::empty::<(&str, &str)>(),
        )
        .await;
    assert!(matches!(
        blocked,
        Err(TransportError::Budget(
            assetrail_lib::connectors::binance::BudgetError::CircuitOpen {
                blocked_until_millis: 4_000
            }
        ))
    ));
}
