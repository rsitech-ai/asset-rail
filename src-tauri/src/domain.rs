use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(Decimal);

impl Amount {
    /// Parses a non-negative exact decimal amount.
    ///
    /// # Errors
    /// Returns [`PlannerError::InvalidAmount`] when the string is not a decimal or is negative.
    pub fn parse(value: &str) -> Result<Self, PlannerError> {
        parse_exact_non_negative_decimal(value)
            .map(Self)
            .ok_or_else(|| PlannerError::InvalidAmount(value.to_owned()))
    }

    /// Subtracts an exact fee from an amount.
    ///
    /// # Errors
    /// Returns [`PlannerError::FeeExceedsAmount`] when subtraction would produce a negative value.
    pub fn checked_sub(&self, other: &Self) -> Result<Self, PlannerError> {
        self.0
            .checked_sub(other.0)
            .filter(|value| !value.is_sign_negative())
            .map(Self)
            .ok_or(PlannerError::FeeExceedsAmount)
    }

    #[must_use]
    pub const fn decimal(&self) -> Decimal {
        self.0
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[must_use]
    pub fn is_multiple_of(&self, increment: &Self) -> bool {
        !increment.0.is_zero() && (self.0 % increment.0).is_zero()
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

pub(crate) fn parse_exact_non_negative_decimal(value: &str) -> Option<Decimal> {
    let (whole, fraction) = value
        .split_once('.')
        .map_or((value, None), |(whole, fraction)| (whole, Some(fraction)));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|fraction| {
            fraction.is_empty()
                || fraction.len() > 28
                || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
        || value.matches('.').count() > 1
    {
        return None;
    }
    let significant_digits = value
        .bytes()
        .filter(u8::is_ascii_digit)
        .skip_while(|byte| *byte == b'0')
        .count();
    if significant_digits > 28 {
        return None;
    }
    Decimal::from_str_exact(value).ok()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub name: String,
}

impl Chain {
    /// Creates a canonical CAIP-2-like chain identity.
    ///
    /// # Errors
    /// Returns [`PlannerError::InvalidCanonicalIdentity`] for an empty name or malformed ID.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Result<Self, PlannerError> {
        let id = id.into();
        let name = name.into();
        if !canonical_chain_id_is_valid(&id) || name.trim().is_empty() {
            return Err(PlannerError::InvalidCanonicalIdentity(id));
        }
        Ok(Self { id, name })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepresentationClass {
    Native,
    IssuerNative,
    CanonicalToken,
    CanonicalBridged,
    ThirdPartyWrapped,
    Synthetic,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRepresentation {
    pub economic_asset: String,
    pub caip19: String,
    pub classification: RepresentationClass,
}

impl AssetRepresentation {
    /// Creates a canonical asset representation.
    ///
    /// # Errors
    /// Returns [`PlannerError::InvalidCanonicalIdentity`] when the asset or CAIP-19 ID is invalid.
    pub fn new(
        economic_asset: impl Into<String>,
        caip19: impl Into<String>,
        classification: RepresentationClass,
    ) -> Result<Self, PlannerError> {
        let economic_asset = economic_asset.into();
        let caip19 = caip19.into();
        if economic_asset.trim().is_empty() || !asset_representation_id_is_valid(&caip19) {
            return Err(PlannerError::InvalidCanonicalIdentity(caip19));
        }
        Ok(Self {
            economic_asset,
            caip19,
            classification,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportConfidence {
    VerifiedByDestinationApi,
    VerifiedByWalletConnection,
    VerifiedBySignedOwnershipProof,
    CuratedByPlatform,
    UserConfirmed,
    Inferred,
    Unknown,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DestinationProfile {
    pub id: String,
    pub name: String,
    pub chain: Chain,
    pub address: String,
    pub supported_representations: Vec<AssetRepresentation>,
    pub memo: Option<String>,
    pub confidence: SupportConfidence,
}

impl std::fmt::Debug for DestinationProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DestinationProfile")
            .field("destination_redacted", &true)
            .field("chain_id", &self.chain.id)
            .field(
                "supported_representation_count",
                &self.supported_representations.len(),
            )
            .field("memo_present", &self.memo.is_some())
            .field("confidence", &self.confidence)
            .finish_non_exhaustive()
    }
}

impl DestinationProfile {
    /// Creates a destination whose chain and asset support are explicit.
    ///
    /// # Errors
    /// Returns [`PlannerError::InvalidDestination`] when identifying fields are empty.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        chain: Chain,
        address: impl Into<String>,
        supported_representations: Vec<AssetRepresentation>,
        memo: Option<String>,
        confidence: SupportConfidence,
    ) -> Result<Self, PlannerError> {
        let id = id.into();
        let name = name.into();
        let address = address.into();
        if id.trim().is_empty()
            || name.trim().is_empty()
            || address.trim().is_empty()
            || supported_representations.iter().any(|representation| {
                representation_chain(&representation.caip19) != Some(&chain.id)
            })
        {
            return Err(PlannerError::InvalidDestination);
        }
        Ok(Self {
            id,
            name,
            chain,
            address,
            supported_representations,
            memo,
            confidence,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MappingStatus {
    Approved,
    PendingReview,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskTier {
    Low,
    Medium,
    High,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkAvailability {
    Available,
    Busy,
    WithdrawalDisabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoPolicy {
    Forbidden,
    Optional,
    Required,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkOption {
    pub exchange_code: String,
    pub chain: Chain,
    pub representation: AssetRepresentation,
    pub availability: NetworkAvailability,
    pub fee: Amount,
    pub minimum: Amount,
    pub maximum: Amount,
    pub increment: Amount,
    pub estimated_minutes: u32,
    pub mapping_status: MappingStatus,
    pub risk_tier: RiskTier,
    pub memo_policy: MemoPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecommendationObjective {
    Safest,
    Cheapest,
    Fastest,
    Balanced,
    NativeOnly,
}

#[derive(Clone, PartialEq, Eq)]
pub struct PlannerInput {
    pub asset_symbol: String,
    pub available: Amount,
    pub withdrawal_amount: Amount,
    pub destination: DestinationProfile,
    pub networks: Vec<NetworkOption>,
    pub objective: RecommendationObjective,
    pub snapshot_age_seconds: u64,
    pub maximum_snapshot_age_seconds: u64,
}

impl std::fmt::Debug for PlannerInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlannerInput")
            .field("asset_symbol", &self.asset_symbol)
            .field("amounts_redacted", &true)
            .field("destination_redacted", &true)
            .field("network_count", &self.networks.len())
            .field("objective", &self.objective)
            .field("snapshot_age_seconds", &self.snapshot_age_seconds)
            .field(
                "maximum_snapshot_age_seconds",
                &self.maximum_snapshot_age_seconds,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EligibleRoute {
    pub exchange_code: String,
    pub gross_amount: Amount,
    pub fee: Amount,
    pub net_received: Amount,
    pub chain: Chain,
    pub representation: AssetRepresentation,
    pub estimated_minutes: u32,
    pub risk_tier: RiskTier,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RouteEvaluation {
    pub eligible: Vec<EligibleRoute>,
    pub excluded: Vec<ExcludedRoute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedRoute {
    pub exchange_code: String,
    pub code: ExclusionCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExclusionCode {
    SnapshotStale,
    InsufficientBalance,
    WithdrawalDisabled,
    NetworkBusy,
    BelowMinimum,
    AboveMaximum,
    InvalidIncrement,
    MappingUnapproved,
    DestinationChainUnsupported,
    DestinationRepresentationUnsupported,
    AddressInvalid,
    MemoRequired,
    MemoUnsupported,
    RepresentationBlocked,
    RiskBlocked,
    ConfidenceInsufficient,
    ObjectiveMismatch,
    NonPositiveWithdrawal,
    NonPositiveNet,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlannerError {
    #[error("invalid decimal amount: {0}")]
    InvalidAmount(String),
    #[error("invalid canonical identity: {0}")]
    InvalidCanonicalIdentity(String),
    #[error("invalid destination profile")]
    InvalidDestination,
    #[error("withdrawal fee exceeds gross amount")]
    FeeExceedsAmount,
    #[error("unknown asset: {0}")]
    UnknownAsset(String),
    #[error("unknown destination: {0}")]
    UnknownDestination(String),
}

fn canonical_chain_id_is_valid(value: &str) -> bool {
    let Some((namespace, reference)) = value.split_once(':') else {
        return false;
    };
    value.matches(':').count() == 1
        && (3..=8).contains(&namespace.len())
        && namespace
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && (1..=32).contains(&reference.len())
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn asset_representation_id_is_valid(value: &str) -> bool {
    let Some((chain, asset)) = value.split_once('/') else {
        return false;
    };
    let Some((namespace, reference)) = asset.split_once(':') else {
        return false;
    };
    value.matches('/').count() == 1
        && canonical_chain_id_is_valid(chain)
        && asset.matches(':').count() == 1
        && (3..=8).contains(&namespace.len())
        && namespace
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && (1..=128).contains(&reference.len())
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'%'))
}

pub(crate) fn representation_chain(caip19: &str) -> Option<&str> {
    caip19.split_once('/').map(|(chain, _)| chain)
}
