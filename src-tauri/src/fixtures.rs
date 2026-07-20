use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::{
    Amount, AssetRepresentation, Chain, DestinationProfile, ExclusionCode, MappingStatus,
    MemoPolicy, NetworkAvailability, NetworkOption, PlannerError, PlannerInput,
    RecommendationObjective, RepresentationClass, RiskTier, SupportConfidence,
};
use crate::route_engine::RouteEngine;

const SNAPSHOT_AT: &str = "2026-07-14T08:45:00Z";
const FRESH_FOR_SECONDS: u64 = 120;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerRequest {
    pub asset_symbol: String,
    pub destination_id: String,
    pub amount: String,
    pub objective: RecommendationObjective,
    pub snapshot_age_seconds: u64,
}

impl Default for PlannerRequest {
    fn default() -> Self {
        Self {
            asset_symbol: "USDC".into(),
            destination_id: "ledger-arbitrum".into(),
            amount: "1000.00".into(),
            objective: RecommendationObjective::Balanced,
            snapshot_age_seconds: 14,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerSnapshot {
    pub mode: String,
    pub provider: String,
    pub generated_at: String,
    pub snapshot_age_seconds: u64,
    pub fresh_for_seconds: u64,
    pub balances: Vec<BalanceView>,
    pub destinations: Vec<DestinationView>,
    pub selection: SelectionView,
    pub eligible: Vec<RouteView>,
    pub excluded: Vec<ExcludedRouteView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceView {
    pub asset_symbol: String,
    pub asset_name: String,
    pub available: String,
    pub locked: String,
    pub fiat_value: String,
    pub fiat_currency: String,
    pub available_networks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationView {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub chain_id: String,
    pub chain_name: String,
    pub address: String,
    pub address_label: String,
    pub confidence: SupportConfidence,
    pub supported_assets: Vec<String>,
    pub memo: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionView {
    pub asset_symbol: String,
    pub destination_id: String,
    pub amount: String,
    pub objective: RecommendationObjective,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteView {
    pub id: String,
    pub exchange_code: String,
    pub network_name: String,
    pub chain_id: String,
    pub representation_id: String,
    pub representation_class: RepresentationClass,
    pub gross_amount: String,
    pub fee: String,
    pub net_received: String,
    pub effective_fee_percent: String,
    pub estimated_minutes: u32,
    pub risk_tier: RiskTier,
    pub confidence: SupportConfidence,
    pub recommended: bool,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedRouteView {
    pub exchange_code: String,
    pub network_name: String,
    pub chain_id: String,
    pub code: ExclusionCode,
    pub title: String,
    pub detail: String,
}

/// Builds a deterministic, non-secret planner snapshot through the production route engine.
///
/// # Errors
/// Returns a typed planner error when request identity or decimal inputs are invalid.
pub fn fixture_planner_snapshot(request: &PlannerRequest) -> Result<PlannerSnapshot, PlannerError> {
    let balances = fixture_balances();
    let balance = balances
        .iter()
        .find(|balance| balance.asset_symbol == request.asset_symbol)
        .ok_or_else(|| PlannerError::UnknownAsset(request.asset_symbol.clone()))?;
    let destination_models = fixture_destinations()?;
    let destination = destination_models
        .iter()
        .find(|(view, _)| view.id == request.destination_id)
        .ok_or_else(|| PlannerError::UnknownDestination(request.destination_id.clone()))?;
    let networks = fixture_networks(&request.asset_symbol)?;
    let input = PlannerInput {
        asset_symbol: request.asset_symbol.clone(),
        available: Amount::parse(&balance.available)?,
        withdrawal_amount: Amount::parse(&request.amount)?,
        destination: destination.1.clone(),
        networks: networks.clone(),
        objective: request.objective,
        snapshot_age_seconds: request.snapshot_age_seconds,
        maximum_snapshot_age_seconds: FRESH_FOR_SECONDS,
    };
    let evaluation = RouteEngine::evaluate(&input)?;
    let eligible = evaluation
        .eligible
        .into_iter()
        .enumerate()
        .map(|(index, route)| RouteView {
            id: format!("{}:{}", request.asset_symbol, route.exchange_code),
            network_name: route.chain.name.clone(),
            chain_id: route.chain.id.clone(),
            representation_id: route.representation.caip19.clone(),
            representation_class: route.representation.classification,
            gross_amount: route.gross_amount.to_string(),
            fee: route.fee.to_string(),
            net_received: route.net_received.to_string(),
            effective_fee_percent: percent(route.fee.decimal(), route.gross_amount.decimal()),
            estimated_minutes: route.estimated_minutes,
            risk_tier: route.risk_tier,
            confidence: destination.1.confidence,
            recommended: index == 0,
            explanation: recommendation_explanation(request.objective, &route.exchange_code),
            exchange_code: route.exchange_code,
        })
        .collect();
    let excluded = evaluation
        .excluded
        .into_iter()
        .map(|excluded| {
            let network = networks
                .iter()
                .find(|network| network.exchange_code == excluded.exchange_code)
                .ok_or_else(|| {
                    PlannerError::InvalidCanonicalIdentity(excluded.exchange_code.clone())
                })?;
            let (title, detail) = exclusion_copy(excluded.code);
            Ok(ExcludedRouteView {
                exchange_code: excluded.exchange_code,
                network_name: network.chain.name.clone(),
                chain_id: network.chain.id.clone(),
                code: excluded.code,
                title: title.into(),
                detail: detail.into(),
            })
        })
        .collect::<Result<Vec<_>, PlannerError>>()?;

    Ok(PlannerSnapshot {
        mode: "OFFLINE_FIXTURE".into(),
        provider: "Binance Spot · deterministic fixture".into(),
        generated_at: SNAPSHOT_AT.into(),
        snapshot_age_seconds: request.snapshot_age_seconds,
        fresh_for_seconds: FRESH_FOR_SECONDS,
        balances,
        destinations: destination_models
            .into_iter()
            .map(|(view, _)| view)
            .collect(),
        selection: SelectionView {
            asset_symbol: request.asset_symbol.clone(),
            destination_id: request.destination_id.clone(),
            amount: request.amount.clone(),
            objective: request.objective,
        },
        eligible,
        excluded,
    })
}

fn fixture_balances() -> Vec<BalanceView> {
    vec![
        BalanceView {
            asset_symbol: "USDC".into(),
            asset_name: "USD Coin".into(),
            available: "1842.50".into(),
            locked: "125.00".into(),
            fiat_value: "1695.10".into(),
            fiat_currency: "EUR".into(),
            available_networks: 3,
        },
        BalanceView {
            asset_symbol: "BTC".into(),
            asset_name: "Bitcoin".into(),
            available: "0.0348".into(),
            locked: "0".into(),
            fiat_value: "3286.41".into(),
            fiat_currency: "EUR".into(),
            available_networks: 2,
        },
        BalanceView {
            asset_symbol: "ETH".into(),
            asset_name: "Ethereum".into(),
            available: "1.284".into(),
            locked: "0.100".into(),
            fiat_value: "3128.22".into(),
            fiat_currency: "EUR".into(),
            available_networks: 1,
        },
        BalanceView {
            asset_symbol: "XRP".into(),
            asset_name: "XRP".into(),
            available: "9250".into(),
            locked: "250".into(),
            fiat_value: "4871.30".into(),
            fiat_currency: "EUR".into(),
            available_networks: 1,
        },
    ]
}

fn fixture_destinations() -> Result<Vec<(DestinationView, DestinationProfile)>, PlannerError> {
    let arbitrum = Chain::new("eip155:42161", "Arbitrum One")?;
    let ethereum = Chain::new("eip155:1", "Ethereum")?;
    let bitcoin = Chain::new("bip122:000000000019d6689c085ae165831e93", "Bitcoin")?;
    let xrpl = Chain::new("xrpl:0", "XRP Ledger")?;
    let usdc_arbitrum = representation(
        "USDC",
        "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        RepresentationClass::CanonicalToken,
    )?;
    let eth = representation("ETH", "eip155:1/slip44:60", RepresentationClass::Native)?;
    let usdc_ethereum = representation(
        "USDC",
        "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        RepresentationClass::IssuerNative,
    )?;
    let btc = representation(
        "BTC",
        "bip122:000000000019d6689c085ae165831e93/slip44:0",
        RepresentationClass::Native,
    )?;
    let xrp = representation("XRP", "xrpl:0/slip44:144", RepresentationClass::Native)?;

    Ok(vec![
        destination(
            "ledger-arbitrum",
            "Hardware wallet A · Arbitrum",
            "Hardware wallet",
            arbitrum,
            "synthetic:wallet-a-arbitrum",
            vec![usdc_arbitrum],
            None,
            SupportConfidence::VerifiedByWalletConnection,
        )?,
        destination(
            "trezor-ethereum",
            "Hardware wallet B · Ethereum",
            "Hardware wallet",
            ethereum,
            "synthetic:wallet-b-ethereum",
            vec![eth, usdc_ethereum],
            None,
            SupportConfidence::VerifiedBySignedOwnershipProof,
        )?,
        destination(
            "cold-bitcoin",
            "Cold vault · Bitcoin",
            "Self-custody",
            bitcoin,
            "synthetic:cold-vault-bitcoin",
            vec![btc],
            None,
            SupportConfidence::VerifiedBySignedOwnershipProof,
        )?,
        destination(
            "kraken-xrp",
            "Simulated exchange · XRP deposit",
            "Exchange / VASP",
            xrpl,
            "synthetic:exchange-xrp",
            vec![xrp],
            None,
            SupportConfidence::VerifiedByDestinationApi,
        )?,
    ])
}

#[allow(clippy::too_many_arguments)]
fn destination(
    id: &str,
    name: &str,
    kind: &str,
    chain: Chain,
    address: &str,
    representations: Vec<AssetRepresentation>,
    memo: Option<String>,
    confidence: SupportConfidence,
) -> Result<(DestinationView, DestinationProfile), PlannerError> {
    let routing_address = fixture_routing_address(&chain);
    let supported_assets = representations
        .iter()
        .map(|representation| representation.economic_asset.clone())
        .collect();
    let address_label = abbreviated_address(address);
    let view = DestinationView {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        chain_id: chain.id.clone(),
        chain_name: chain.name.clone(),
        address: address.into(),
        address_label,
        confidence,
        supported_assets,
        memo: memo.clone(),
    };
    let profile = DestinationProfile::new(
        id,
        name,
        chain,
        routing_address,
        representations,
        memo,
        confidence,
    )?;
    Ok((view, profile))
}

fn fixture_routing_address(chain: &Chain) -> String {
    // These private values exercise the route engine's exact, fail-closed fixture allowlist.
    // DestinationView exposes separate synthetic tokens, so no address-like test value reaches
    // the public UI or serialized snapshot.
    match chain.id.as_str() {
        "eip155:42161" => "0x1111111111111111111111111111111111111111".into(),
        "eip155:1" => "0x2222222222222222222222222222222222222222".into(),
        "bip122:000000000019d6689c085ae165831e93" => {
            "bc1qassetrail9pc3x9d4v8g0f7l5r2n6k3m2p7s8u9q".into()
        }
        "xrpl:0" => "rAssetRailExampleAddress123456789".into(),
        _ => "unsupported-routing-fixture".into(),
    }
}

fn fixture_networks(asset: &str) -> Result<Vec<NetworkOption>, PlannerError> {
    match asset {
        "USDC" => usdc_networks(),
        "BTC" => btc_networks(),
        "ETH" => eth_networks(),
        "XRP" => xrp_networks(),
        other => Err(PlannerError::UnknownAsset(other.into())),
    }
}

fn usdc_networks() -> Result<Vec<NetworkOption>, PlannerError> {
    Ok(vec![
        network(
            "ARBITRUM",
            Chain::new("eip155:42161", "Arbitrum One")?,
            representation(
                "USDC",
                "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
                RepresentationClass::CanonicalToken,
            )?,
            "0.15",
            "1",
            "100000",
            "0.01",
            NetworkAvailability::Available,
            MappingStatus::Approved,
            RiskTier::Low,
            2,
            MemoPolicy::Forbidden,
        )?,
        network(
            "ETH",
            Chain::new("eip155:1", "Ethereum")?,
            representation(
                "USDC",
                "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                RepresentationClass::IssuerNative,
            )?,
            "3.20",
            "10",
            "100000",
            "0.01",
            NetworkAvailability::Available,
            MappingStatus::Approved,
            RiskTier::Low,
            8,
            MemoPolicy::Forbidden,
        )?,
        network(
            "BSC",
            Chain::new("eip155:56", "BNB Smart Chain")?,
            representation(
                "USDC",
                "eip155:56/bep20:0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
                RepresentationClass::ThirdPartyWrapped,
            )?,
            "0.20",
            "5",
            "100000",
            "0.01",
            NetworkAvailability::Available,
            MappingStatus::PendingReview,
            RiskTier::High,
            3,
            MemoPolicy::Forbidden,
        )?,
    ])
}

fn btc_networks() -> Result<Vec<NetworkOption>, PlannerError> {
    Ok(vec![
        network(
            "BTC",
            Chain::new("bip122:000000000019d6689c085ae165831e93", "Bitcoin")?,
            representation(
                "BTC",
                "bip122:000000000019d6689c085ae165831e93/slip44:0",
                RepresentationClass::Native,
            )?,
            "0.00005",
            "0.0001",
            "100",
            "0.00000001",
            NetworkAvailability::Available,
            MappingStatus::Approved,
            RiskTier::Low,
            35,
            MemoPolicy::Forbidden,
        )?,
        network(
            "LIGHTNING",
            Chain::new(
                "bip122:000000000019d6689c085ae165831e93",
                "Bitcoin Lightning",
            )?,
            representation(
                "BTC",
                "bip122:000000000019d6689c085ae165831e93/slip44:0",
                RepresentationClass::Native,
            )?,
            "0.000001",
            "0.00001",
            "0.1",
            "0.00000001",
            NetworkAvailability::Available,
            MappingStatus::PendingReview,
            RiskTier::Medium,
            1,
            MemoPolicy::Forbidden,
        )?,
    ])
}

fn eth_networks() -> Result<Vec<NetworkOption>, PlannerError> {
    Ok(vec![network(
        "ETH",
        Chain::new("eip155:1", "Ethereum")?,
        representation("ETH", "eip155:1/slip44:60", RepresentationClass::Native)?,
        "0.0012",
        "0.01",
        "1000",
        "0.00000001",
        NetworkAvailability::Available,
        MappingStatus::Approved,
        RiskTier::Low,
        8,
        MemoPolicy::Forbidden,
    )?])
}

fn xrp_networks() -> Result<Vec<NetworkOption>, PlannerError> {
    Ok(vec![network(
        "XRP",
        Chain::new("xrpl:0", "XRP Ledger")?,
        representation("XRP", "xrpl:0/slip44:144", RepresentationClass::Native)?,
        "0.20",
        "20",
        "10000000",
        "0.000001",
        NetworkAvailability::Available,
        MappingStatus::Approved,
        RiskTier::Low,
        1,
        MemoPolicy::Required,
    )?])
}

#[allow(clippy::too_many_arguments)]
fn network(
    exchange_code: &str,
    chain: Chain,
    representation: AssetRepresentation,
    fee: &str,
    minimum: &str,
    maximum: &str,
    increment: &str,
    availability: NetworkAvailability,
    mapping_status: MappingStatus,
    risk_tier: RiskTier,
    estimated_minutes: u32,
    memo_policy: MemoPolicy,
) -> Result<NetworkOption, PlannerError> {
    Ok(NetworkOption {
        exchange_code: exchange_code.into(),
        chain,
        representation,
        availability,
        fee: Amount::parse(fee)?,
        minimum: Amount::parse(minimum)?,
        maximum: Amount::parse(maximum)?,
        increment: Amount::parse(increment)?,
        estimated_minutes,
        mapping_status,
        risk_tier,
        memo_policy,
    })
}

fn representation(
    asset: &str,
    caip19: &str,
    class: RepresentationClass,
) -> Result<AssetRepresentation, PlannerError> {
    AssetRepresentation::new(asset, caip19, class)
}

fn abbreviated_address(address: &str) -> String {
    if address.starts_with("synthetic:") {
        return "Synthetic fixture".into();
    }
    if address.len() <= 18 {
        return address.into();
    }
    format!("{}…{}", &address[..10], &address[address.len() - 6..])
}

fn percent(numerator: Decimal, denominator: Decimal) -> String {
    if denominator.is_zero() {
        return "0".into();
    }
    (numerator * Decimal::ONE_HUNDRED / denominator)
        .round_dp(4)
        .normalize()
        .to_string()
}

fn recommendation_explanation(objective: RecommendationObjective, network: &str) -> String {
    match objective {
        RecommendationObjective::Safest => {
            format!(
                "{network} has the lowest eligible risk tier for this simulated destination evidence."
            )
        }
        RecommendationObjective::Cheapest => {
            format!("{network} has the lowest exact withdrawal fee among eligible routes.")
        }
        RecommendationObjective::Fastest => {
            format!("{network} has the shortest exchange arrival estimate among eligible routes.")
        }
        RecommendationObjective::Balanced => format!(
            "{network} leads on risk first, then fee and expected arrival time as deterministic tie-breakers."
        ),
        RecommendationObjective::NativeOnly => {
            format!("{network} uses the native or issuer-native asset representation.")
        }
    }
}

const fn exclusion_copy(code: ExclusionCode) -> (&'static str, &'static str) {
    match code {
        ExclusionCode::SnapshotStale => (
            "Snapshot expired",
            "Refresh critical exchange data before using this route.",
        ),
        ExclusionCode::InsufficientBalance => (
            "Insufficient available balance",
            "Reduce the amount or wait for locked funds to become available.",
        ),
        ExclusionCode::WithdrawalDisabled => (
            "Withdrawals unavailable",
            "The exchange currently disables withdrawal on this network.",
        ),
        ExclusionCode::NetworkBusy => (
            "Network busy",
            "Workspace policy blocks busy networks until service stabilizes.",
        ),
        ExclusionCode::BelowMinimum => (
            "Below exchange minimum",
            "Increase the amount to satisfy the network minimum.",
        ),
        ExclusionCode::AboveMaximum => (
            "Above exchange maximum",
            "Reduce the amount to the network maximum or lower.",
        ),
        ExclusionCode::InvalidIncrement => (
            "Invalid amount increment",
            "Use an amount aligned to the exchange withdrawal increment.",
        ),
        ExclusionCode::MappingUnapproved => (
            "Canonical mapping not approved",
            "AssetRail cannot prove this exchange code maps to the intended chain and token.",
        ),
        ExclusionCode::DestinationChainUnsupported => (
            "Destination chain mismatch",
            "The fixture destination evidence maps to a different canonical chain.",
        ),
        ExclusionCode::DestinationRepresentationUnsupported => (
            "Token representation unsupported",
            "The destination does not explicitly support this exact CAIP-19 asset.",
        ),
        ExclusionCode::AddressInvalid => (
            "Address invalid for chain",
            "The address fails validation for the explicitly selected chain.",
        ),
        ExclusionCode::MemoRequired => (
            "Memo required",
            "Add the destination memo or tag before planning this route.",
        ),
        ExclusionCode::MemoUnsupported => (
            "Memo not supported",
            "Remove the memo because this network does not accept one.",
        ),
        ExclusionCode::RepresentationBlocked => (
            "Representation blocked",
            "Unknown asset representations are not eligible for recommendation.",
        ),
        ExclusionCode::RiskBlocked => (
            "Route blocked by risk policy",
            "This route is not eligible under the current risk policy.",
        ),
        ExclusionCode::ConfidenceInsufficient => (
            "Destination evidence insufficient",
            "Verify destination support through a stronger evidence source.",
        ),
        ExclusionCode::ObjectiveMismatch => (
            "Not a native representation",
            "Native-only mode excludes token and wrapped representations.",
        ),
        ExclusionCode::NonPositiveWithdrawal => (
            "Withdrawal amount must be positive",
            "Enter an amount greater than zero before planning a route.",
        ),
        ExclusionCode::NonPositiveNet => (
            "Fee consumes the withdrawal",
            "Increase the amount so the exact net received remains positive.",
        ),
    }
}
