use assetrail_lib::domain::{
    Amount, AssetRepresentation, Chain, DestinationProfile, ExclusionCode, MappingStatus,
    MemoPolicy, NetworkAvailability, NetworkOption, PlannerInput, RecommendationObjective,
    RepresentationClass, RiskTier, SupportConfidence,
};
use assetrail_lib::route_engine::RouteEngine;
use proptest::prelude::*;

fn amount(value: &str) -> Amount {
    Amount::parse(value).expect("test amount must be valid")
}

#[test]
fn planner_amounts_reject_non_fixed_point_and_lossy_inputs() {
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
            Amount::parse(invalid).is_err(),
            "{invalid:?} must not cross the exact-decimal boundary"
        );
    }
}

fn eligible_input() -> PlannerInput {
    let chain = Chain::new("eip155:42161", "Arbitrum One").expect("valid chain");
    let representation = AssetRepresentation::new(
        "USDC",
        "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        RepresentationClass::CanonicalToken,
    )
    .expect("valid representation");
    let network = NetworkOption {
        exchange_code: "ARBITRUM".into(),
        chain: chain.clone(),
        representation: representation.clone(),
        availability: NetworkAvailability::Available,
        fee: amount("0.15"),
        minimum: amount("1"),
        maximum: amount("100000"),
        increment: amount("0.01"),
        estimated_minutes: 2,
        mapping_status: MappingStatus::Approved,
        risk_tier: RiskTier::Low,
        memo_policy: MemoPolicy::Forbidden,
    };
    let destination = DestinationProfile::new(
        "ledger",
        "Ledger Vault",
        chain,
        "0x1111111111111111111111111111111111111111",
        vec![representation],
        None,
        SupportConfidence::VerifiedByWalletConnection,
    )
    .expect("valid destination");
    PlannerInput {
        asset_symbol: "USDC".into(),
        available: amount("1250.50"),
        withdrawal_amount: amount("100.00"),
        destination,
        networks: vec![network],
        objective: RecommendationObjective::Balanced,
        snapshot_age_seconds: 12,
        maximum_snapshot_age_seconds: 120,
    }
}

#[test]
fn supported_enabled_network_returns_exact_net_amount() {
    let input = eligible_input();

    let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");

    assert_eq!(result.eligible.len(), 1);
    assert_eq!(result.eligible[0].gross_amount.to_string(), "100.00");
    assert_eq!(result.eligible[0].fee.to_string(), "0.15");
    assert_eq!(result.eligible[0].net_received.to_string(), "99.85");
    assert!(result.excluded.is_empty());
}

#[test]
fn stale_snapshot_excludes_every_route() {
    let mut input = eligible_input();
    input.snapshot_age_seconds = 121;

    let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");

    assert!(result.eligible.is_empty());
    assert_eq!(result.excluded.len(), 1);
    assert_eq!(result.excluded[0].code, ExclusionCode::SnapshotStale);
}

#[test]
fn unapproved_mapping_is_excluded_before_destination_checks() {
    let mut input = eligible_input();
    input.networks[0].mapping_status = MappingStatus::PendingReview;
    input.destination.chain = Chain::new("eip155:1", "Ethereum").expect("valid chain");

    let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");

    assert!(result.eligible.is_empty());
    assert_eq!(result.excluded[0].code, ExclusionCode::MappingUnapproved);
}

#[test]
fn evm_shaped_address_cannot_substitute_for_explicit_chain_support() {
    let mut input = eligible_input();
    input.destination.chain = Chain::new("eip155:1", "Ethereum").expect("valid chain");

    let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");

    assert!(result.eligible.is_empty());
    assert_eq!(
        result.excluded[0].code,
        ExclusionCode::DestinationChainUnsupported
    );
}

#[test]
fn destination_must_support_the_exact_asset_representation() {
    let mut input = eligible_input();
    input.destination.supported_representations.clear();

    let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");

    assert!(result.eligible.is_empty());
    assert_eq!(
        result.excluded[0].code,
        ExclusionCode::DestinationRepresentationUnsupported
    );
}

#[test]
fn requested_economic_asset_must_match_the_network_representation() {
    let mut input = eligible_input();
    input.asset_symbol = "BTC".into();

    assert_eq!(only_exclusion(&input), ExclusionCode::RepresentationBlocked);
}

#[test]
fn representation_chain_must_match_the_network_chain() {
    let mut input = eligible_input();
    input.networks[0].representation = AssetRepresentation::new(
        "USDC",
        "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        RepresentationClass::IssuerNative,
    )
    .expect("valid representation on a different chain");
    input.destination.supported_representations[0] = input.networks[0].representation.clone();

    assert_eq!(only_exclusion(&input), ExclusionCode::RepresentationBlocked);
}

#[test]
fn canonical_domain_constructors_reject_empty_identity_components() {
    assert!(Chain::new(":", "Invalid").is_err());
    assert!(Chain::new("eip155:", "Invalid").is_err());
    assert!(AssetRepresentation::new("USDC", "/", RepresentationClass::CanonicalToken).is_err());
    assert!(
        AssetRepresentation::new("USDC", "eip155:1/", RepresentationClass::CanonicalToken).is_err()
    );
}

fn only_exclusion(input: &PlannerInput) -> ExclusionCode {
    let result = RouteEngine::evaluate(input).expect("route evaluation succeeds");
    assert!(result.eligible.is_empty());
    assert_eq!(result.excluded.len(), 1);
    result.excluded[0].code
}

#[test]
fn operational_and_amount_rules_exclude_before_cost_ranking() {
    let mut insufficient = eligible_input();
    insufficient.available = amount("99.99");
    assert_eq!(
        only_exclusion(&insufficient),
        ExclusionCode::InsufficientBalance
    );

    let mut disabled = eligible_input();
    disabled.networks[0].availability = NetworkAvailability::WithdrawalDisabled;
    assert_eq!(only_exclusion(&disabled), ExclusionCode::WithdrawalDisabled);

    let mut busy = eligible_input();
    busy.networks[0].availability = NetworkAvailability::Busy;
    assert_eq!(only_exclusion(&busy), ExclusionCode::NetworkBusy);

    let mut below_minimum = eligible_input();
    below_minimum.networks[0].minimum = amount("100.01");
    assert_eq!(only_exclusion(&below_minimum), ExclusionCode::BelowMinimum);

    let mut above_maximum = eligible_input();
    above_maximum.networks[0].maximum = amount("99.99");
    assert_eq!(only_exclusion(&above_maximum), ExclusionCode::AboveMaximum);

    let mut invalid_increment = eligible_input();
    invalid_increment.withdrawal_amount = amount("100.005");
    assert_eq!(
        only_exclusion(&invalid_increment),
        ExclusionCode::InvalidIncrement
    );

    let mut zero_increment = eligible_input();
    zero_increment.networks[0].increment = amount("0");
    assert_eq!(
        only_exclusion(&zero_increment),
        ExclusionCode::InvalidIncrement
    );

    let mut zero_withdrawal = eligible_input();
    zero_withdrawal.withdrawal_amount = amount("0");
    zero_withdrawal.networks[0].minimum = amount("0");
    let zero_withdrawal_result =
        RouteEngine::evaluate(&zero_withdrawal).expect("route evaluation succeeds");
    assert!(zero_withdrawal_result.eligible.is_empty());

    let mut zero_net = eligible_input();
    zero_net.networks[0].fee = zero_net.withdrawal_amount.clone();
    let zero_net_result = RouteEngine::evaluate(&zero_net).expect("route evaluation succeeds");
    assert!(zero_net_result.eligible.is_empty());
}

#[test]
fn identity_memo_and_confidence_rules_fail_closed() {
    let mut invalid_address = eligible_input();
    invalid_address.destination.address = "0x1234".into();
    assert_eq!(
        only_exclusion(&invalid_address),
        ExclusionCode::AddressInvalid
    );

    let mut memo_required = eligible_input();
    memo_required.networks[0].memo_policy = MemoPolicy::Required;
    assert_eq!(only_exclusion(&memo_required), ExclusionCode::MemoRequired);

    let mut memo_unsupported = eligible_input();
    memo_unsupported.destination.memo = Some("12345".into());
    assert_eq!(
        only_exclusion(&memo_unsupported),
        ExclusionCode::MemoUnsupported
    );

    let mut unknown_representation = eligible_input();
    unknown_representation.networks[0]
        .representation
        .classification = RepresentationClass::Unknown;
    assert_eq!(
        only_exclusion(&unknown_representation),
        ExclusionCode::RepresentationBlocked
    );

    let mut inferred_destination = eligible_input();
    inferred_destination.destination.confidence = SupportConfidence::Inferred;
    assert_eq!(
        only_exclusion(&inferred_destination),
        ExclusionCode::ConfidenceInsufficient
    );
}

#[test]
fn recommendation_objective_deterministically_orders_eligible_routes() {
    let mut input = eligible_input();
    let mut economical = input.networks[0].clone();
    economical.exchange_code = "ARBITRUM_ECONOMY".into();
    economical.fee = amount("0.05");
    economical.estimated_minutes = 8;
    economical.risk_tier = RiskTier::Medium;
    input.networks.push(economical);
    input.objective = RecommendationObjective::Cheapest;

    let cheapest = RouteEngine::evaluate(&input).expect("route evaluation succeeds");
    assert_eq!(cheapest.eligible[0].exchange_code, "ARBITRUM_ECONOMY");

    input.objective = RecommendationObjective::Fastest;
    let fastest = RouteEngine::evaluate(&input).expect("route evaluation succeeds");
    assert_eq!(fastest.eligible[0].exchange_code, "ARBITRUM");

    input.objective = RecommendationObjective::Safest;
    let safest = RouteEngine::evaluate(&input).expect("route evaluation succeeds");
    assert_eq!(safest.eligible[0].exchange_code, "ARBITRUM");
}

#[test]
fn native_only_objective_does_not_recommend_token_representations() {
    let mut input = eligible_input();
    input.objective = RecommendationObjective::NativeOnly;

    assert_eq!(only_exclusion(&input), ExclusionCode::ObjectiveMismatch);
}

#[test]
fn blocked_risk_tier_is_never_eligible_for_recommendation() {
    let mut input = eligible_input();
    input.networks[0].risk_tier = RiskTier::Blocked;

    assert_eq!(only_exclusion(&input), ExclusionCode::RiskBlocked);
}

proptest! {
    #[test]
    fn eligible_route_never_creates_or_loses_value(
        gross_cents in 100_u64..10_000_000,
        fee_cents in 0_u64..99,
    ) {
        let mut input = eligible_input();
        input.withdrawal_amount = amount(&format!("{}.{:02}", gross_cents / 100, gross_cents % 100));
        input.available = input.withdrawal_amount.clone();
        input.networks[0].fee = amount(&format!("0.{fee_cents:02}"));
        input.networks[0].minimum = amount("0.01");
        input.networks[0].maximum = amount("1000000");
        input.networks[0].increment = amount("0.01");

        let result = RouteEngine::evaluate(&input).expect("route evaluation succeeds");
        let route = &result.eligible[0];

        prop_assert_eq!(
            route.net_received.decimal() + route.fee.decimal(),
            route.gross_amount.decimal()
        );
        prop_assert!(route.net_received.decimal().is_sign_positive());
        prop_assert_eq!(route.chain.id.as_str(), input.destination.chain.id.as_str());
        prop_assert!(input.destination.supported_representations.iter().any(
            |representation| representation.caip19 == route.representation.caip19
        ));
        prop_assert_eq!(input.networks[0].mapping_status, MappingStatus::Approved);
    }
}
