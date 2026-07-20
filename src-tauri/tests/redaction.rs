use assetrail_lib::security::{
    RedactedAddress, RedactedCredential, RedactedEvent, RedactedSignature,
};
use assetrail_lib::{
    connectors::{
        BalanceRecord, CanonicalChainId, ConnectorId, DecimalAmount, DestinationKind,
        DestinationProfileEnvelope, DestinationSource, ExchangeSnapshot, PermissionPosture,
        SavedAddressCandidate, SourceAccountId, SystemStatus, Timestamp, VerificationState,
    },
    domain::{
        Amount, AssetRepresentation, Chain, DestinationProfile, PlannerInput,
        RecommendationObjective, RepresentationClass, SupportConfidence,
    },
};
use uuid::Uuid;

#[test]
fn diagnostics_redact_credentials_addresses_and_signatures() {
    let rendered = RedactedEvent::from_sensitive_fixture().to_string();

    for forbidden in ["fixture-secret", "X-MBX-APIKEY", "signature=", "0x529084"] {
        assert!(!rendered.contains(forbidden));
    }
}

#[test]
fn sensitive_diagnostic_wrappers_have_fixed_redacted_formatting() {
    let values: Vec<Box<dyn std::fmt::Display>> = vec![
        Box::new(RedactedCredential::new("fixture-secret")),
        Box::new(RedactedAddress::new(
            "0x52908400098527886E0F7030069857D2E4169EE7",
        )),
        Box::new(RedactedSignature::new("signature=fixture-signature")),
    ];

    for value in values {
        assert_eq!(value.to_string(), "[REDACTED]");
    }
}

#[test]
fn sensitive_aggregate_debug_output_is_allowlist_redacted() {
    let timestamp = Timestamp::parse("2026-07-17T12:00:00Z").expect("valid timestamp");
    let balance = BalanceRecord {
        provider_asset_code: "USDC".into(),
        free: DecimalAmount::parse("918273.645").expect("valid canary balance"),
        locked: DecimalAmount::parse("0").expect("valid amount"),
        frozen: DecimalAmount::parse("0").expect("valid amount"),
        withdrawing: DecimalAmount::parse("0").expect("valid amount"),
    };
    let candidate = SavedAddressCandidate {
        provider_asset_code: "USDC".into(),
        provider_network_code: Some("ARBITRUM".into()),
        address: "0xfeed00000000000000000000000000000000beef".into(),
        memo: Some("memo-canary-7321".into()),
    };
    let snapshot = ExchangeSnapshot {
        schema_version: 1,
        id: Uuid::new_v4(),
        connector_id: ConnectorId::parse("binance-spot").expect("valid connector"),
        source_account_id: SourceAccountId::new(),
        observed_at: timestamp.clone(),
        permissions: PermissionPosture::read_only(timestamp.clone()),
        balances: vec![balance],
        networks: Vec::new(),
        system_status: SystemStatus::Normal,
        withdrawal_quota: None,
        saved_address_candidates: vec![candidate],
        reference_prices: Vec::new(),
    };
    let envelope = DestinationProfileEnvelope {
        schema_version: 1,
        id: Uuid::new_v4(),
        label: "private-vault-label".into(),
        kind: DestinationKind::SelfCustody,
        source: DestinationSource::Manual,
        canonical_chain_id: CanonicalChainId::parse("eip155:1").expect("valid chain"),
        canonical_address: "0xfeed00000000000000000000000000000000beef".into(),
        memo: Some("memo-canary-7321".into()),
        verification_state: VerificationState::UserConfirmed,
        verification_evidence_hash: None,
        verified_at: Some(timestamp.clone()),
        expires_at: None,
        asset_support: Vec::new(),
    };
    let chain = Chain::new("eip155:1", "Ethereum").expect("valid chain");
    let representation = AssetRepresentation::new(
        "USDC",
        "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        RepresentationClass::IssuerNative,
    )
    .expect("valid representation");
    let destination = DestinationProfile::new(
        "private-destination-id",
        "private-destination-label",
        chain,
        "0xfeed00000000000000000000000000000000beef",
        vec![representation],
        Some("memo-canary-7321".into()),
        SupportConfidence::UserConfirmed,
    )
    .expect("valid destination");
    let planner_input = PlannerInput {
        asset_symbol: "USDC".into(),
        available: Amount::parse("918273.645").expect("valid balance"),
        withdrawal_amount: Amount::parse("42").expect("valid amount"),
        destination,
        networks: Vec::new(),
        objective: RecommendationObjective::Balanced,
        snapshot_age_seconds: 1,
        maximum_snapshot_age_seconds: 120,
    };

    let rendered = format!("{snapshot:?}\n{envelope:?}\n{planner_input:?}");
    for forbidden in [
        "918273.645",
        "0xfeed00000000000000000000000000000000beef",
        "memo-canary-7321",
        "private-vault-label",
        "private-destination-label",
    ] {
        assert!(!rendered.contains(forbidden), "leaked {forbidden:?}");
    }
    assert!(rendered.contains("balance_count"));
    assert!(rendered.contains("destination_redacted"));
}
