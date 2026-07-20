use assetrail_lib::connectors::{
    AssetRepresentationId, CanonicalAssetId, CanonicalChainId, ConnectorError, ConnectorId,
    ConnectorRegistry, DecimalAmount, DestinationProfileEnvelope, ExchangeConnector,
    ExchangeSnapshot, MappingRecord, PermissionPosture, PermissionState, QuoteEnvelope,
    ReadCapability, RegistryError, SourceAccountId, Timestamp,
};
use assetrail_lib::security::{CredentialHandle, CredentialMode, Planning};
use std::sync::Arc;

struct FakeConnector {
    id: ConnectorId,
}

#[async_trait::async_trait]
impl ExchangeConnector for FakeConnector {
    fn id(&self) -> &ConnectorId {
        &self.id
    }

    fn capabilities(&self) -> &[ReadCapability] {
        &[ReadCapability::PermissionPosture]
    }

    async fn inspect_permissions(
        &self,
        _credential: &CredentialHandle<Planning>,
    ) -> Result<PermissionPosture, ConnectorError> {
        Ok(PermissionPosture::read_only(
            Timestamp::parse("2026-07-14T18:00:00Z").expect("valid timestamp"),
        ))
    }

    async fn synchronize(
        &self,
        _credential: &CredentialHandle<Planning>,
    ) -> Result<ExchangeSnapshot, ConnectorError> {
        Err(ConnectorError::Unavailable)
    }
}

#[test]
fn planning_credentials_report_planning_mode() {
    assert_eq!(
        CredentialHandle::<Planning>::mode_for_type(),
        CredentialMode::Planning
    );
}

#[test]
fn planning_credentials_cannot_be_retyped_for_execution() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/planning_cannot_execute.rs");
}

#[test]
fn connector_identifiers_are_normalized_at_construction() {
    assert!(ConnectorId::parse("binance-spot").is_ok());
    assert!(ConnectorId::parse("Binance Spot").is_err());
    assert!(ConnectorId::parse("").is_err());
}

#[test]
fn planning_permission_posture_fails_closed() {
    let inspected_at = Timestamp::parse("2026-07-14T18:00:00Z").expect("valid timestamp");
    let mut posture = PermissionPosture::read_only(inspected_at);
    assert!(posture.is_planning_safe());

    posture.withdrawal = PermissionState::Enabled;
    assert!(!posture.is_planning_safe());

    posture.withdrawal = PermissionState::Disabled;
    posture.trading = PermissionState::Unknown;
    assert!(!posture.is_planning_safe());
}

#[test]
fn connector_registry_rejects_duplicate_identifiers() {
    let mut registry = ConnectorRegistry::new();
    let id = ConnectorId::parse("binance-spot").expect("valid connector identifier");

    registry
        .register(Arc::new(FakeConnector { id: id.clone() }))
        .expect("first registration succeeds");
    let duplicate = registry.register(Arc::new(FakeConnector { id: id.clone() }));

    assert_eq!(duplicate, Err(RegistryError::Duplicate(id.clone())));
    assert_eq!(registry.get(&id).expect("registered connector").id(), &id);
}

#[test]
fn canonical_value_objects_revalidate_during_deserialization() {
    assert!(serde_json::from_str::<ConnectorId>(r#""Binance Spot""#).is_err());
    assert!(serde_json::from_str::<assetrail_lib::connectors::DecimalAmount>(r#""-1""#).is_err());
    assert!(serde_json::from_str::<Timestamp>(r#""not-a-timestamp""#).is_err());
}

#[test]
fn account_ids_canonical_ids_and_timestamps_are_normalized() {
    let account = SourceAccountId::new();
    assert_eq!(
        SourceAccountId::parse(account.as_str()).expect("generated account ID round-trips"),
        account
    );
    assert!(SourceAccountId::parse("provider-account-name").is_err());

    assert!(CanonicalAssetId::parse("usdc").is_ok());
    assert!(CanonicalAssetId::parse("USDC").is_err());
    assert!(CanonicalChainId::parse("eip155:42161").is_ok());
    assert!(CanonicalChainId::parse("Ethereum").is_err());
    assert!(
        AssetRepresentationId::parse(
            "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831"
        )
        .is_ok()
    );
    assert!(AssetRepresentationId::parse("USDC-on-Arbitrum").is_err());

    let offset = Timestamp::parse("2026-07-14T20:00:00+02:00").expect("valid offset timestamp");
    let utc = Timestamp::parse("2026-07-14T18:00:00Z").expect("equivalent UTC timestamp");
    assert_eq!(offset, utc);
    assert_eq!(offset.as_str(), "2026-07-14T18:00:00Z");
    assert!(Timestamp::parse("2026-07-14T18:00:00.1Z").unwrap() > utc);
}

#[test]
fn decimal_amounts_are_canonical_and_use_numeric_ordering() {
    let one = DecimalAmount::parse("1.0").expect("valid decimal");
    let equivalent_one = DecimalAmount::parse("1.00").expect("valid equivalent decimal");
    let two = DecimalAmount::parse("2").expect("valid decimal");
    let ten = DecimalAmount::parse("10").expect("valid decimal");

    assert_eq!(one, equivalent_one);
    assert_eq!(one.as_str(), "1");
    assert!(two < ten);
}

#[test]
fn decimal_amounts_reject_non_fixed_point_and_lossy_inputs() {
    for invalid in [
        "1e3",
        "1_000",
        "+1",
        ".1",
        "1.",
        "0.12345678901234567890123456789",
        "12345678901234567890123456789",
    ] {
        assert!(
            DecimalAmount::parse(invalid).is_err(),
            "{invalid:?} must not cross the exact-decimal boundary"
        );
    }
}

#[test]
fn canonical_identity_value_objects_reject_empty_components() {
    for invalid in [":", "eip155:", ":1", "eip155:1:extra"] {
        assert!(CanonicalChainId::parse(invalid).is_err());
    }
    for invalid in ["/", "eip155:1/", "/erc20:token"] {
        assert!(AssetRepresentationId::parse(invalid).is_err());
    }
}

fn valid_mapping_json() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "id": "10000000-0000-4000-8000-000000000001",
        "connectorId": "binance-spot",
        "providerAssetCode": "USDC",
        "providerNetworkCode": "ARBITRUM",
        "canonicalAssetId": "usdc",
        "canonicalChainId": "eip155:42161",
        "assetRepresentationId": "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        "state": "APPROVED",
        "sourceVersion": "fixture-v1",
        "reviewedAt": "2026-07-17T12:00:00Z"
    })
}

#[test]
fn versioned_envelopes_reject_unknown_schema_versions() {
    let mut value = valid_mapping_json();
    value["schemaVersion"] = serde_json::json!(2);

    assert!(serde_json::from_value::<MappingRecord>(value).is_err());
}

#[test]
fn approved_mapping_rejects_cross_chain_representation() {
    let mut value = valid_mapping_json();
    value["assetRepresentationId"] =
        serde_json::json!("eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

    assert!(serde_json::from_value::<MappingRecord>(value).is_err());
}

fn valid_destination_json() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "id": "20000000-0000-4000-8000-000000000002",
        "label": "fixture destination",
        "kind": "SELF_CUSTODY",
        "source": "MANUAL",
        "canonicalChainId": "eip155:42161",
        "canonicalAddress": "0x1111111111111111111111111111111111111111",
        "memo": null,
        "verificationState": "USER_CONFIRMED",
        "verificationEvidenceHash": null,
        "verifiedAt": "2026-07-17T12:00:00Z",
        "expiresAt": "2026-07-18T12:00:00Z",
        "assetSupport": [{
            "assetRepresentationId": "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
            "allowed": true,
            "verificationState": "USER_CONFIRMED"
        }]
    })
}

#[test]
fn destination_envelopes_reject_invalid_or_unsupported_chain_addresses() {
    assert!(serde_json::from_value::<DestinationProfileEnvelope>(valid_destination_json()).is_ok());

    for address in ["", "fixture-redacted-at-boundary", "0x1234"] {
        let mut value = valid_destination_json();
        value["canonicalAddress"] = serde_json::json!(address);
        assert!(serde_json::from_value::<DestinationProfileEnvelope>(value).is_err());
    }

    let mut unsupported = valid_destination_json();
    unsupported["canonicalChainId"] = serde_json::json!("solana:mainnet");
    unsupported["assetSupport"][0]["assetRepresentationId"] =
        serde_json::json!("solana:mainnet/slip44:501");
    assert!(serde_json::from_value::<DestinationProfileEnvelope>(unsupported).is_err());
}

#[test]
fn destination_envelopes_reject_cross_chain_support_and_reverse_expiry() {
    let mut cross_chain = valid_destination_json();
    cross_chain["assetSupport"][0]["assetRepresentationId"] =
        serde_json::json!("eip155:1/slip44:60");
    assert!(serde_json::from_value::<DestinationProfileEnvelope>(cross_chain).is_err());

    let mut reverse_expiry = valid_destination_json();
    reverse_expiry["expiresAt"] = serde_json::json!("2026-07-17T11:59:59Z");
    assert!(serde_json::from_value::<DestinationProfileEnvelope>(reverse_expiry).is_err());
}

#[test]
fn quote_envelopes_reject_incoherent_arithmetic_time_and_status() {
    let value = serde_json::json!({
        "schemaVersion": 1,
        "id": "30000000-0000-4000-8000-000000000003",
        "exchangeSnapshotId": "30000000-0000-4000-8000-000000000004",
        "canonicalSnapshotHash": "fixture-hash",
        "sourceAccountId": "30000000-0000-4000-8000-000000000005",
        "destinationId": "30000000-0000-4000-8000-000000000006",
        "canonicalChainId": "eip155:42161",
        "assetRepresentationId": "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        "objective": "BALANCED",
        "status": "ELIGIBLE",
        "grossAmount": "100",
        "feeAmount": "1",
        "netAmount": "100",
        "priceEvidenceIds": [],
        "reasonCodes": ["CONTRADICTORY_REASON"],
        "createdAt": "2026-07-17T12:00:00Z",
        "expiresAt": "2026-07-17T11:59:59Z"
    });

    assert!(serde_json::from_value::<QuoteEnvelope>(value).is_err());
}
