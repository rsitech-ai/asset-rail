use assetrail_lib::domain::{ExclusionCode, RecommendationObjective};
use assetrail_lib::fixtures::{PlannerRequest, fixture_planner_snapshot};

#[test]
fn usdc_planner_snapshot_explains_valid_and_ambiguous_networks() {
    let request = PlannerRequest {
        asset_symbol: "USDC".into(),
        destination_id: "ledger-arbitrum".into(),
        amount: "1000.00".into(),
        objective: RecommendationObjective::Cheapest,
        snapshot_age_seconds: 14,
    };

    let snapshot = fixture_planner_snapshot(&request).expect("fixture planner succeeds");

    assert_eq!(snapshot.mode, "OFFLINE_FIXTURE");
    assert_eq!(snapshot.selection.asset_symbol, "USDC");
    assert!(
        snapshot
            .balances
            .iter()
            .all(|balance| balance.available != "0")
    );
    assert_eq!(snapshot.eligible.len(), 1);
    assert_eq!(snapshot.eligible[0].exchange_code, "ARBITRUM");
    assert_eq!(snapshot.eligible[0].net_received, "999.85");
    assert!(snapshot.eligible[0].recommended);
    assert!(
        !snapshot.eligible[0]
            .explanation
            .contains("verified destination")
    );
    assert!(snapshot.excluded.iter().any(|route| {
        route.exchange_code == "ETH" && route.code == ExclusionCode::DestinationChainUnsupported
    }));
    assert!(snapshot.excluded.iter().any(|route| {
        route.exchange_code == "BSC" && route.code == ExclusionCode::MappingUnapproved
    }));
}

#[test]
fn advertised_network_counts_match_fixture_route_metadata() {
    let request = PlannerRequest {
        asset_symbol: "ETH".into(),
        destination_id: "trezor-ethereum".into(),
        amount: "1.00".into(),
        objective: RecommendationObjective::Balanced,
        snapshot_age_seconds: 1,
    };

    let snapshot = fixture_planner_snapshot(&request).expect("fixture planner succeeds");
    let eth = snapshot
        .balances
        .iter()
        .find(|balance| balance.asset_symbol == "ETH")
        .expect("ETH fixture balance exists");

    assert_eq!(eth.available_networks, 1);
    assert_eq!(snapshot.eligible.len() + snapshot.excluded.len(), 1);
}

#[test]
fn public_fixtures_expose_synthetic_destination_tokens_instead_of_address_or_memo_data() {
    let snapshot =
        fixture_planner_snapshot(&PlannerRequest::default()).expect("fixture planner succeeds");

    assert_eq!(snapshot.destinations.len(), 4);
    for destination in snapshot.destinations {
        assert!(destination.address.starts_with("synthetic:"));
        assert_eq!(destination.address_label, "Synthetic fixture");
        assert_eq!(destination.memo, None);
    }
}
