use crate::domain::{
    EligibleRoute, ExcludedRoute, ExclusionCode, MappingStatus, MemoPolicy, NetworkAvailability,
    NetworkOption, PlannerError, PlannerInput, RecommendationObjective, RepresentationClass,
    RiskTier, RouteEvaluation, SupportConfidence,
};

pub struct RouteEngine;

impl RouteEngine {
    /// Evaluates direct-withdrawal candidates by applying hard filters before ranking.
    ///
    /// # Errors
    /// Returns [`PlannerError::FeeExceedsAmount`] if an otherwise eligible route has an invalid fee.
    pub fn evaluate(input: &PlannerInput) -> Result<RouteEvaluation, PlannerError> {
        let mut evaluation = RouteEvaluation::default();
        for network in &input.networks {
            if let Some(code) = first_exclusion(input, network) {
                evaluation.excluded.push(ExcludedRoute {
                    exchange_code: network.exchange_code.clone(),
                    code,
                });
                continue;
            }
            evaluation.eligible.push(EligibleRoute {
                exchange_code: network.exchange_code.clone(),
                gross_amount: input.withdrawal_amount.clone(),
                fee: network.fee.clone(),
                net_received: input.withdrawal_amount.checked_sub(&network.fee)?,
                chain: network.chain.clone(),
                representation: network.representation.clone(),
                estimated_minutes: network.estimated_minutes,
                risk_tier: network.risk_tier,
            });
        }
        sort_eligible(&mut evaluation.eligible, input.objective);
        Ok(evaluation)
    }
}

fn first_exclusion(input: &PlannerInput, network: &NetworkOption) -> Option<ExclusionCode> {
    if input.snapshot_age_seconds > input.maximum_snapshot_age_seconds {
        return Some(ExclusionCode::SnapshotStale);
    }
    if input.withdrawal_amount.is_zero() {
        return Some(ExclusionCode::NonPositiveWithdrawal);
    }
    if input.withdrawal_amount > input.available {
        return Some(ExclusionCode::InsufficientBalance);
    }
    match network.availability {
        NetworkAvailability::WithdrawalDisabled => {
            return Some(ExclusionCode::WithdrawalDisabled);
        }
        NetworkAvailability::Busy => return Some(ExclusionCode::NetworkBusy),
        NetworkAvailability::Available => {}
    }
    if input.withdrawal_amount < network.minimum {
        return Some(ExclusionCode::BelowMinimum);
    }
    if input.withdrawal_amount > network.maximum {
        return Some(ExclusionCode::AboveMaximum);
    }
    if !input.withdrawal_amount.is_multiple_of(&network.increment) {
        return Some(ExclusionCode::InvalidIncrement);
    }
    if network.mapping_status != MappingStatus::Approved {
        return Some(ExclusionCode::MappingUnapproved);
    }
    if network.risk_tier == RiskTier::Blocked {
        return Some(ExclusionCode::RiskBlocked);
    }
    if network.representation.economic_asset != input.asset_symbol
        || crate::domain::representation_chain(&network.representation.caip19)
            != Some(network.chain.id.as_str())
    {
        return Some(ExclusionCode::RepresentationBlocked);
    }
    if network.chain.id != input.destination.chain.id {
        return Some(ExclusionCode::DestinationChainUnsupported);
    }
    if !input
        .destination
        .supported_representations
        .iter()
        .any(|representation| representation.caip19 == network.representation.caip19)
    {
        return Some(ExclusionCode::DestinationRepresentationUnsupported);
    }
    if network.fee == input.withdrawal_amount {
        return Some(ExclusionCode::NonPositiveNet);
    }
    if !address_is_supported_fixture_for_chain(&network.chain.id, &input.destination.address) {
        return Some(ExclusionCode::AddressInvalid);
    }
    let has_memo = input
        .destination
        .memo
        .as_deref()
        .is_some_and(|memo| !memo.trim().is_empty());
    if network.memo_policy == MemoPolicy::Required && !has_memo {
        return Some(ExclusionCode::MemoRequired);
    }
    if network.memo_policy == MemoPolicy::Forbidden && has_memo {
        return Some(ExclusionCode::MemoUnsupported);
    }
    if network.representation.classification == RepresentationClass::Unknown {
        return Some(ExclusionCode::RepresentationBlocked);
    }
    if input.objective == RecommendationObjective::NativeOnly
        && !matches!(
            network.representation.classification,
            RepresentationClass::Native | RepresentationClass::IssuerNative
        )
    {
        return Some(ExclusionCode::ObjectiveMismatch);
    }
    if matches!(
        input.destination.confidence,
        SupportConfidence::Inferred | SupportConfidence::Unknown
    ) {
        return Some(ExclusionCode::ConfidenceInsufficient);
    }
    None
}

fn sort_eligible(routes: &mut [EligibleRoute], objective: RecommendationObjective) {
    routes.sort_by(|left, right| match objective {
        RecommendationObjective::Cheapest => left
            .fee
            .cmp(&right.fee)
            .then_with(|| risk_rank(left.risk_tier).cmp(&risk_rank(right.risk_tier)))
            .then_with(|| left.estimated_minutes.cmp(&right.estimated_minutes))
            .then_with(|| left.exchange_code.cmp(&right.exchange_code)),
        RecommendationObjective::Fastest => left
            .estimated_minutes
            .cmp(&right.estimated_minutes)
            .then_with(|| risk_rank(left.risk_tier).cmp(&risk_rank(right.risk_tier)))
            .then_with(|| left.fee.cmp(&right.fee))
            .then_with(|| left.exchange_code.cmp(&right.exchange_code)),
        RecommendationObjective::Safest
        | RecommendationObjective::Balanced
        | RecommendationObjective::NativeOnly => risk_rank(left.risk_tier)
            .cmp(&risk_rank(right.risk_tier))
            .then_with(|| left.fee.cmp(&right.fee))
            .then_with(|| left.estimated_minutes.cmp(&right.estimated_minutes))
            .then_with(|| left.exchange_code.cmp(&right.exchange_code)),
    });
}

const fn risk_rank(risk_tier: RiskTier) -> u8 {
    match risk_tier {
        RiskTier::Low => 0,
        RiskTier::Medium => 1,
        RiskTier::High => 2,
        RiskTier::Blocked => 3,
    }
}

/// The offline planner uses synthetic fixture addresses that are not proof of live-chain
/// validity. Until audited chain parsers are introduced, accept only those exact fixtures and
/// fail closed for every other address or chain family.
fn address_is_supported_fixture_for_chain(chain_id: &str, address: &str) -> bool {
    if chain_id == "eip155:42161" {
        return address == "0x1111111111111111111111111111111111111111";
    }
    if chain_id == "eip155:1" {
        return address == "0x2222222222222222222222222222222222222222";
    }
    if chain_id == "bip122:000000000019d6689c085ae165831e93" {
        return address == "bc1qassetrail9pc3x9d4v8g0f7l5r2n6k3m2p7s8u9q";
    }
    if chain_id == "xrpl:0" {
        return address == "rAssetRailExampleAddress123456789";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::address_is_supported_fixture_for_chain;

    #[test]
    fn unsupported_synthetic_address_shapes_fail_closed() {
        for (chain_id, address) in [
            (
                "bip122:000000000019d6689c085ae165831e93",
                "bc1!!!!!!!!!!!!!!!!!!!!!!!",
            ),
            ("xrpl:0", "r!!!!!!!!!!!!!!!!!!!!!!!!"),
            ("tron:mainnet", "T!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"),
            ("solana:mainnet", "11111111111111111111111111111111"),
        ] {
            assert!(!address_is_supported_fixture_for_chain(chain_id, address));
        }
    }

    #[test]
    fn current_non_evm_fixture_addresses_remain_supported() {
        assert!(address_is_supported_fixture_for_chain(
            "eip155:42161",
            "0x1111111111111111111111111111111111111111",
        ));
        assert!(address_is_supported_fixture_for_chain(
            "eip155:1",
            "0x2222222222222222222222222222222222222222",
        ));
        assert!(address_is_supported_fixture_for_chain(
            "bip122:000000000019d6689c085ae165831e93",
            "bc1qassetrail9pc3x9d4v8g0f7l5r2n6k3m2p7s8u9q",
        ));
        assert!(address_is_supported_fixture_for_chain(
            "xrpl:0",
            "rAssetRailExampleAddress123456789",
        ));
    }
}
