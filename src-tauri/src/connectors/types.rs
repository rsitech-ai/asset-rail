use std::{cmp::Ordering, fmt};

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use uuid::Uuid;

const CONNECTOR_SCHEMA_VERSION: u16 = 1;

fn deserialize_schema_version<'de, DeserializerType>(
    deserializer: DeserializerType,
) -> Result<u16, DeserializerType::Error>
where
    DeserializerType: Deserializer<'de>,
{
    let version = u16::deserialize(deserializer)?;
    if version != CONNECTOR_SCHEMA_VERSION {
        return Err(DeserializerType::Error::custom(
            "unsupported connector envelope schema version",
        ));
    }
    Ok(version)
}

fn representation_chain_id(representation: &AssetRepresentationId) -> &str {
    representation
        .as_str()
        .split_once('/')
        .map_or("", |(chain, _)| chain)
}

fn canonical_address_is_valid(chain_id: &CanonicalChainId, address: &str) -> bool {
    if address.len() > 128 || address.trim() != address || address.is_empty() {
        return false;
    }
    match chain_id
        .as_str()
        .split_once(':')
        .map(|(namespace, _)| namespace)
    {
        Some("eip155") => {
            address.len() == 42
                && address.starts_with("0x")
                && address[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
        }
        // Additional chain families remain fail-closed until audited parsers are introduced.
        _ => false,
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ConnectorId(
    #[schemars(
        length(min = 1, max = 64),
        regex(pattern = r"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$")
    )]
    String,
);

impl ConnectorId {
    /// Parses a stable connector identifier such as `binance-spot`.
    ///
    /// # Errors
    /// Returns [`IdentifierError`] unless the value is 1 to 64 lowercase ASCII letters,
    /// digits, or hyphens and begins and ends with a letter or digit.
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        let valid_edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let valid_body = |byte: u8| valid_edge(byte) || byte == b'-';
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 64
            || !valid_edge(bytes[0])
            || !valid_edge(bytes[bytes.len() - 1])
            || !bytes.iter().copied().all(valid_body)
        {
            return Err(IdentifierError::InvalidConnectorId);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for ConnectorId {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(DeserializerType::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalAssetId(
    #[schemars(
        length(min = 1, max = 128),
        regex(pattern = r"^[a-z0-9](?:[a-z0-9._-]*[a-z0-9])?$")
    )]
    String,
);

impl CanonicalAssetId {
    /// Parses a provider-independent lowercase economic-asset identifier.
    ///
    /// # Errors
    /// Returns [`IdentifierError`] unless the value is normalized lowercase ASCII.
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        let bytes = value.as_bytes();
        let edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let body = |byte: u8| edge(byte) || matches!(byte, b'.' | b'_' | b'-');
        if bytes.is_empty()
            || bytes.len() > 128
            || !edge(bytes[0])
            || !edge(bytes[bytes.len() - 1])
            || !bytes.iter().copied().all(body)
        {
            return Err(IdentifierError::InvalidCanonicalAssetId);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CanonicalAssetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for CanonicalAssetId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalChainId(
    #[schemars(
        length(min = 5, max = 41),
        regex(pattern = r"^[-a-z0-9]{3,8}:[-_a-zA-Z0-9]{1,32}$")
    )]
    String,
);

impl CanonicalChainId {
    /// Parses a CAIP-2 chain identifier.
    ///
    /// # Errors
    /// Returns [`IdentifierError`] unless the namespace and reference satisfy CAIP-2 bounds.
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        let Some((namespace, reference)) = value.split_once(':') else {
            return Err(IdentifierError::InvalidCanonicalChainId);
        };
        if value.matches(':').count() != 1
            || !(3..=8).contains(&namespace.len())
            || !namespace
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || !(1..=32).contains(&reference.len())
            || !reference
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(IdentifierError::InvalidCanonicalChainId);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CanonicalChainId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for CanonicalChainId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AssetRepresentationId(
    #[schemars(
        length(min = 11, max = 181),
        regex(
            pattern = r"^[-a-z0-9]{3,8}:[-_a-zA-Z0-9]{1,32}/[-a-z0-9]{3,8}:[-_.%a-zA-Z0-9]{1,128}$"
        )
    )]
    String,
);

impl AssetRepresentationId {
    /// Parses a CAIP-19 asset representation identifier.
    ///
    /// # Errors
    /// Returns [`IdentifierError`] unless the chain and asset components are canonical.
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        let Some((chain, asset)) = value.split_once('/') else {
            return Err(IdentifierError::InvalidAssetRepresentationId);
        };
        if value.matches('/').count() != 1 || CanonicalChainId::parse(chain).is_err() {
            return Err(IdentifierError::InvalidAssetRepresentationId);
        }
        let Some((namespace, reference)) = asset.split_once(':') else {
            return Err(IdentifierError::InvalidAssetRepresentationId);
        };
        if asset.matches(':').count() != 1
            || !(3..=8).contains(&namespace.len())
            || !namespace
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || !(1..=128).contains(&reference.len())
            || !reference.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'%')
            })
        {
            return Err(IdentifierError::InvalidAssetRepresentationId);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetRepresentationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for AssetRepresentationId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceAccountId(
    #[schemars(
        length(equal = 36),
        regex(pattern = r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
    )]
    String,
);

impl SourceAccountId {
    /// Generates an opaque, non-secret source-account identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Parses an opaque UUID source-account identifier.
    ///
    /// # Errors
    /// Returns [`IdentifierError`] unless the value is a canonical hyphenated UUID.
    pub fn parse(value: &str) -> Result<Self, IdentifierError> {
        let parsed = Uuid::parse_str(value).map_err(|_| IdentifierError::InvalidSourceAccountId)?;
        if parsed.to_string() != value {
            return Err(IdentifierError::InvalidSourceAccountId);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SourceAccountId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SourceAccountId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for SourceAccountId {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(DeserializerType::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentifierError {
    #[error("connector identifier must be normalized lowercase ASCII")]
    InvalidConnectorId,
    #[error("source account identifier must be a canonical hyphenated UUID")]
    InvalidSourceAccountId,
    #[error("canonical asset identifier must be normalized lowercase ASCII")]
    InvalidCanonicalAssetId,
    #[error("canonical chain identifier must satisfy CAIP-2 bounds")]
    InvalidCanonicalChainId,
    #[error("asset representation identifier must satisfy CAIP-19 bounds")]
    InvalidAssetRepresentationId,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Timestamp(
    #[schemars(
        length(min = 20, max = 35),
        regex(
            pattern = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?(?:Z|[+-][0-9]{2}:[0-9]{2})$"
        )
    )]
    String,
);

impl Timestamp {
    /// Parses an RFC 3339 timestamp and normalizes it to UTC.
    ///
    /// # Errors
    /// Returns [`TimestampError`] when the value is not valid RFC 3339.
    pub fn parse(value: &str) -> Result<Self, TimestampError> {
        let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| TimestampError)?;
        let normalized = parsed
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .map_err(|_| TimestampError)?;
        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        let left =
            OffsetDateTime::parse(&self.0, &Rfc3339).expect("Timestamp stores valid RFC 3339");
        let right =
            OffsetDateTime::parse(&other.0, &Rfc3339).expect("Timestamp stores valid RFC 3339");
        left.cmp(&right)
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(DeserializerType::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("timestamp must be RFC 3339 with an explicit UTC offset")]
pub struct TimestampError;

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PermissionState {
    Disabled,
    Enabled,
    Unknown,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionPosture {
    pub reading: PermissionState,
    pub withdrawal: PermissionState,
    pub trading: PermissionState,
    pub margin: PermissionState,
    pub futures: PermissionState,
    pub internal_transfer: PermissionState,
    pub ip_restricted: bool,
    pub unknown_permissions: Vec<String>,
    pub inspected_at: Timestamp,
}

impl PermissionPosture {
    #[must_use]
    pub fn read_only(inspected_at: Timestamp) -> Self {
        Self {
            reading: PermissionState::Enabled,
            withdrawal: PermissionState::Disabled,
            trading: PermissionState::Disabled,
            margin: PermissionState::Disabled,
            futures: PermissionState::Disabled,
            internal_transfer: PermissionState::Disabled,
            ip_restricted: false,
            unknown_permissions: Vec::new(),
            inspected_at,
        }
    }

    #[must_use]
    pub fn is_planning_safe(&self) -> bool {
        self.reading == PermissionState::Enabled
            && self.withdrawal == PermissionState::Disabled
            && self.trading == PermissionState::Disabled
            && self.margin == PermissionState::Disabled
            && self.futures == PermissionState::Disabled
            && self.internal_transfer == PermissionState::Disabled
            && self.unknown_permissions.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReadCapability {
    PermissionPosture,
    SpotBalances,
    WithdrawalNetworks,
    ServerTime,
    SystemStatus,
    WithdrawalQuota,
    MarketCatalog,
    ReferencePrices,
    SavedWithdrawalAddresses,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DecimalAmount(#[schemars(regex(pattern = r"^(?:0|[0-9]+)(?:\.[0-9]+)?$"))] String);

impl DecimalAmount {
    /// Parses a non-negative exact decimal and stores its canonical string representation.
    ///
    /// # Errors
    /// Returns [`DecimalAmountError`] for malformed or negative values.
    pub fn parse(value: &str) -> Result<Self, DecimalAmountError> {
        let parsed =
            crate::domain::parse_exact_non_negative_decimal(value).ok_or(DecimalAmountError)?;
        Ok(Self(parsed.normalize().to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the exact parsed decimal.
    ///
    /// # Errors
    /// This can fail only if a deserializer bypassed the validated constructor. Callers should
    /// reject the enclosing boundary object when that occurs.
    pub fn decimal(&self) -> Result<Decimal, DecimalAmountError> {
        Decimal::from_str_exact(&self.0).map_err(|_| DecimalAmountError)
    }
}

impl Ord for DecimalAmount {
    fn cmp(&self, other: &Self) -> Ordering {
        let left = Decimal::from_str_exact(&self.0).expect("DecimalAmount stores a valid decimal");
        let right =
            Decimal::from_str_exact(&other.0).expect("DecimalAmount stores a valid decimal");
        left.cmp(&right)
    }
}

impl PartialOrd for DecimalAmount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for DecimalAmount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for DecimalAmount {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(DeserializerType::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("amount must be a non-negative exact decimal string")]
pub struct DecimalAmountError;

#[derive(Clone, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BalanceRecord {
    pub provider_asset_code: String,
    pub free: DecimalAmount,
    pub locked: DecimalAmount,
    pub frozen: DecimalAmount,
    pub withdrawing: DecimalAmount,
}

impl fmt::Debug for BalanceRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BalanceRecord")
            .field("provider_asset_code", &self.provider_asset_code)
            .field("amounts_redacted", &true)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoRequirement {
    Forbidden,
    Optional,
    Required,
    Unknown,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkMetadataRecord {
    pub provider_asset_code: String,
    pub provider_network_code: String,
    pub withdrawal_enabled: bool,
    pub busy: bool,
    pub fee: DecimalAmount,
    pub minimum: DecimalAmount,
    pub maximum: Option<DecimalAmount>,
    pub increment: DecimalAmount,
    pub memo_requirement: MemoRequirement,
    pub estimated_arrival_minutes: Option<u32>,
    pub contract_address: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemStatus {
    Normal,
    Maintenance,
    Unknown,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WithdrawalQuota {
    pub used: DecimalAmount,
    pub limit: DecimalAmount,
    pub resets_at: Option<Timestamp>,
}

#[derive(Clone, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedAddressCandidate {
    pub provider_asset_code: String,
    pub provider_network_code: Option<String>,
    pub address: String,
    pub memo: Option<String>,
}

impl fmt::Debug for SavedAddressCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SavedAddressCandidate")
            .field("provider_asset_code", &self.provider_asset_code)
            .field("provider_network_code", &self.provider_network_code)
            .field("address_redacted", &true)
            .field("memo_present", &self.memo.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
pub enum QuoteCurrency {
    EUR,
    USD,
    PLN,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceConfidence {
    DirectMarket,
    Derived,
    Unavailable,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferencePriceRecord {
    pub id: Uuid,
    pub provider_asset_code: String,
    pub quote_currency: QuoteCurrency,
    pub price: DecimalAmount,
    pub observed_at: Timestamp,
    pub confidence: PriceConfidence,
}

#[derive(Clone, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExchangeSnapshot {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u16,
    pub id: Uuid,
    pub connector_id: ConnectorId,
    pub source_account_id: SourceAccountId,
    pub observed_at: Timestamp,
    pub permissions: PermissionPosture,
    pub balances: Vec<BalanceRecord>,
    pub networks: Vec<NetworkMetadataRecord>,
    pub system_status: SystemStatus,
    pub withdrawal_quota: Option<WithdrawalQuota>,
    pub saved_address_candidates: Vec<SavedAddressCandidate>,
    pub reference_prices: Vec<ReferencePriceRecord>,
}

impl fmt::Debug for ExchangeSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExchangeSnapshot")
            .field("schema_version", &self.schema_version)
            .field("id", &self.id)
            .field("connector_id", &self.connector_id)
            .field("source_account_id", &self.source_account_id)
            .field("observed_at", &self.observed_at)
            .field("balance_count", &self.balances.len())
            .field("network_count", &self.networks.len())
            .field("system_status", &self.system_status)
            .field("withdrawal_quota_present", &self.withdrawal_quota.is_some())
            .field(
                "saved_address_candidate_count",
                &self.saved_address_candidates.len(),
            )
            .field("reference_price_count", &self.reference_prices.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConnectorError {
    #[error("connector authentication failed")]
    Authentication,
    #[error("connector permission posture is unsafe or unavailable")]
    Authorization,
    #[error("connector clock synchronization failed")]
    Clock,
    #[error("connector rate limit reached")]
    RateLimited,
    #[error("connector is unavailable")]
    Unavailable,
    #[error("connector response did not match the expected schema")]
    Schema,
    #[error("connector data could not be mapped canonically")]
    Mapping,
    #[error("connector failed; diagnostic details were redacted")]
    InternalRedacted,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialScheme {
    HmacSha256,
    Rsa,
    Ed25519,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportState {
    Supported,
    Unsupported,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialSchemeCapability {
    pub scheme: CredentialScheme,
    pub state: SupportState,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectorCapabilityEnvelope {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u16,
    pub connector_id: ConnectorId,
    pub read_capabilities: Vec<ReadCapability>,
    pub credential_schemes: Vec<CredentialSchemeCapability>,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MappingState {
    Approved,
    PendingReview,
    Rejected,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MappingRecord {
    pub schema_version: u16,
    pub id: Uuid,
    pub connector_id: ConnectorId,
    pub provider_asset_code: String,
    pub provider_network_code: String,
    pub canonical_asset_id: CanonicalAssetId,
    pub canonical_chain_id: CanonicalChainId,
    pub asset_representation_id: AssetRepresentationId,
    pub state: MappingState,
    pub source_version: String,
    pub reviewed_at: Option<Timestamp>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MappingRecordUnchecked {
    #[serde(deserialize_with = "deserialize_schema_version")]
    schema_version: u16,
    id: Uuid,
    connector_id: ConnectorId,
    provider_asset_code: String,
    provider_network_code: String,
    canonical_asset_id: CanonicalAssetId,
    canonical_chain_id: CanonicalChainId,
    asset_representation_id: AssetRepresentationId,
    state: MappingState,
    source_version: String,
    reviewed_at: Option<Timestamp>,
}

impl<'de> Deserialize<'de> for MappingRecord {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let raw = MappingRecordUnchecked::deserialize(deserializer)?;
        if representation_chain_id(&raw.asset_representation_id) != raw.canonical_chain_id.as_str()
        {
            return Err(DeserializerType::Error::custom(
                "mapping representation chain does not match canonical chain",
            ));
        }
        if raw.state == MappingState::Approved && raw.reviewed_at.is_none() {
            return Err(DeserializerType::Error::custom(
                "approved mappings require a review timestamp",
            ));
        }
        Ok(Self {
            schema_version: raw.schema_version,
            id: raw.id,
            connector_id: raw.connector_id,
            provider_asset_code: raw.provider_asset_code,
            provider_network_code: raw.provider_network_code,
            canonical_asset_id: raw.canonical_asset_id,
            canonical_chain_id: raw.canonical_chain_id,
            asset_representation_id: raw.asset_representation_id,
            state: raw.state,
            source_version: raw.source_version,
            reviewed_at: raw.reviewed_at,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DestinationKind {
    SelfCustody,
    Exchange,
    Contract,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DestinationSource {
    Manual,
    WalletConnection,
    ProviderImport,
    DestinationApi,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationState {
    Unverified,
    StructurallyValid,
    UserConfirmed,
    OwnershipVerified,
    ProviderConfirmed,
    Expired,
    Rejected,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DestinationAssetSupport {
    pub asset_representation_id: AssetRepresentationId,
    pub allowed: bool,
    pub verification_state: VerificationState,
}

#[derive(Clone, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DestinationProfileEnvelope {
    pub schema_version: u16,
    pub id: Uuid,
    pub label: String,
    pub kind: DestinationKind,
    pub source: DestinationSource,
    pub canonical_chain_id: CanonicalChainId,
    #[schemars(length(min = 1, max = 128), regex(pattern = r"^\S+$"))]
    pub canonical_address: String,
    pub memo: Option<String>,
    pub verification_state: VerificationState,
    pub verification_evidence_hash: Option<String>,
    pub verified_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub asset_support: Vec<DestinationAssetSupport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DestinationProfileEnvelopeUnchecked {
    #[serde(deserialize_with = "deserialize_schema_version")]
    schema_version: u16,
    id: Uuid,
    label: String,
    kind: DestinationKind,
    source: DestinationSource,
    canonical_chain_id: CanonicalChainId,
    canonical_address: String,
    memo: Option<String>,
    verification_state: VerificationState,
    verification_evidence_hash: Option<String>,
    verified_at: Option<Timestamp>,
    expires_at: Option<Timestamp>,
    asset_support: Vec<DestinationAssetSupport>,
}

impl<'de> Deserialize<'de> for DestinationProfileEnvelope {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let raw = DestinationProfileEnvelopeUnchecked::deserialize(deserializer)?;
        if !canonical_address_is_valid(&raw.canonical_chain_id, &raw.canonical_address) {
            return Err(DeserializerType::Error::custom(
                "destination address is invalid for canonical chain",
            ));
        }
        if raw.asset_support.iter().any(|support| {
            representation_chain_id(&support.asset_representation_id)
                != raw.canonical_chain_id.as_str()
        }) {
            return Err(DeserializerType::Error::custom(
                "destination asset support chain does not match canonical chain",
            ));
        }
        if raw
            .verified_at
            .as_ref()
            .zip(raw.expires_at.as_ref())
            .is_some_and(|(verified_at, expires_at)| expires_at <= verified_at)
        {
            return Err(DeserializerType::Error::custom(
                "destination expiry must follow verification",
            ));
        }
        if matches!(
            raw.verification_state,
            VerificationState::UserConfirmed
                | VerificationState::OwnershipVerified
                | VerificationState::ProviderConfirmed
        ) && raw.verified_at.is_none()
        {
            return Err(DeserializerType::Error::custom(
                "verified destinations require a verification timestamp",
            ));
        }
        Ok(Self {
            schema_version: raw.schema_version,
            id: raw.id,
            label: raw.label,
            kind: raw.kind,
            source: raw.source,
            canonical_chain_id: raw.canonical_chain_id,
            canonical_address: raw.canonical_address,
            memo: raw.memo,
            verification_state: raw.verification_state,
            verification_evidence_hash: raw.verification_evidence_hash,
            verified_at: raw.verified_at,
            expires_at: raw.expires_at,
            asset_support: raw.asset_support,
        })
    }
}

impl fmt::Debug for DestinationProfileEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DestinationProfileEnvelope")
            .field("schema_version", &self.schema_version)
            .field("id", &self.id)
            .field("destination_redacted", &true)
            .field("kind", &self.kind)
            .field("source", &self.source)
            .field("canonical_chain_id", &self.canonical_chain_id)
            .field("memo_present", &self.memo.is_some())
            .field("verification_state", &self.verification_state)
            .field("verified_at", &self.verified_at)
            .field("expires_at", &self.expires_at)
            .field("asset_support_count", &self.asset_support.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuoteObjective {
    Safest,
    Cheapest,
    Fastest,
    Balanced,
    NativeOnly,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuoteStatus {
    Eligible,
    Ineligible,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuoteEnvelope {
    pub schema_version: u16,
    pub id: Uuid,
    pub exchange_snapshot_id: Uuid,
    pub canonical_snapshot_hash: String,
    pub source_account_id: SourceAccountId,
    pub destination_id: Uuid,
    pub canonical_chain_id: CanonicalChainId,
    pub asset_representation_id: AssetRepresentationId,
    pub objective: QuoteObjective,
    pub status: QuoteStatus,
    pub gross_amount: DecimalAmount,
    pub fee_amount: DecimalAmount,
    pub net_amount: DecimalAmount,
    pub price_evidence_ids: Vec<Uuid>,
    pub reason_codes: Vec<String>,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QuoteEnvelopeUnchecked {
    #[serde(deserialize_with = "deserialize_schema_version")]
    schema_version: u16,
    id: Uuid,
    exchange_snapshot_id: Uuid,
    canonical_snapshot_hash: String,
    source_account_id: SourceAccountId,
    destination_id: Uuid,
    canonical_chain_id: CanonicalChainId,
    asset_representation_id: AssetRepresentationId,
    objective: QuoteObjective,
    status: QuoteStatus,
    gross_amount: DecimalAmount,
    fee_amount: DecimalAmount,
    net_amount: DecimalAmount,
    price_evidence_ids: Vec<Uuid>,
    reason_codes: Vec<String>,
    created_at: Timestamp,
    expires_at: Timestamp,
}

impl<'de> Deserialize<'de> for QuoteEnvelope {
    fn deserialize<DeserializerType>(
        deserializer: DeserializerType,
    ) -> Result<Self, DeserializerType::Error>
    where
        DeserializerType: Deserializer<'de>,
    {
        let raw = QuoteEnvelopeUnchecked::deserialize(deserializer)?;
        if representation_chain_id(&raw.asset_representation_id) != raw.canonical_chain_id.as_str()
        {
            return Err(DeserializerType::Error::custom(
                "quote representation chain does not match canonical chain",
            ));
        }
        if raw.expires_at <= raw.created_at {
            return Err(DeserializerType::Error::custom(
                "quote expiry must follow creation",
            ));
        }
        let gross = raw
            .gross_amount
            .decimal()
            .map_err(DeserializerType::Error::custom)?;
        let fee = raw
            .fee_amount
            .decimal()
            .map_err(DeserializerType::Error::custom)?;
        let net = raw
            .net_amount
            .decimal()
            .map_err(DeserializerType::Error::custom)?;
        if gross.checked_sub(fee) != Some(net) {
            return Err(DeserializerType::Error::custom(
                "quote net amount must equal gross amount minus fee",
            ));
        }
        if (raw.status == QuoteStatus::Eligible && (!raw.reason_codes.is_empty() || net.is_zero()))
            || (raw.status == QuoteStatus::Ineligible && raw.reason_codes.is_empty())
        {
            return Err(DeserializerType::Error::custom(
                "quote status and reason codes are contradictory",
            ));
        }
        Ok(Self {
            schema_version: raw.schema_version,
            id: raw.id,
            exchange_snapshot_id: raw.exchange_snapshot_id,
            canonical_snapshot_hash: raw.canonical_snapshot_hash,
            source_account_id: raw.source_account_id,
            destination_id: raw.destination_id,
            canonical_chain_id: raw.canonical_chain_id,
            asset_representation_id: raw.asset_representation_id,
            objective: raw.objective,
            status: raw.status,
            gross_amount: raw.gross_amount,
            fee_amount: raw.fee_amount,
            net_amount: raw.net_amount,
            price_evidence_ids: raw.price_evidence_ids,
            reason_codes: raw.reason_codes,
            created_at: raw.created_at,
            expires_at: raw.expires_at,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditOutcome {
    Succeeded,
    Rejected,
    FailedRedacted,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEventEnvelope {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u16,
    pub id: Uuid,
    pub event_name: String,
    pub occurred_at: Timestamp,
    pub outcome: AuditOutcome,
    pub correlation_id: Uuid,
    pub connector_id: Option<ConnectorId>,
    pub source_account_id: Option<SourceAccountId>,
    pub endpoint_class: Option<String>,
    pub result_category: String,
    pub record_count: Option<u64>,
}
