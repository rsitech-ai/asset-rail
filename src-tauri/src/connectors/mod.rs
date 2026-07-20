pub mod binance;
mod registry;
mod types;

use crate::security::{CredentialHandle, Planning};

pub use registry::{ConnectorRegistry, RegistryError};
pub use types::{
    AssetRepresentationId, AuditEventEnvelope, AuditOutcome, BalanceRecord, CanonicalAssetId,
    CanonicalChainId, ConnectorCapabilityEnvelope, ConnectorError, ConnectorId, CredentialScheme,
    CredentialSchemeCapability, DecimalAmount, DecimalAmountError, DestinationAssetSupport,
    DestinationKind, DestinationProfileEnvelope, DestinationSource, ExchangeSnapshot,
    IdentifierError, MappingRecord, MappingState, MemoRequirement, NetworkMetadataRecord,
    PermissionPosture, PermissionState, PriceConfidence, QuoteCurrency, QuoteEnvelope,
    QuoteObjective, QuoteStatus, ReadCapability, ReferencePriceRecord, SavedAddressCandidate,
    SourceAccountId, SupportState, SystemStatus, Timestamp, TimestampError, VerificationState,
    WithdrawalQuota,
};

#[async_trait::async_trait]
pub trait ExchangeConnector: Send + Sync {
    fn id(&self) -> &ConnectorId;
    fn capabilities(&self) -> &[ReadCapability];
    async fn inspect_permissions(
        &self,
        credential: &CredentialHandle<Planning>,
    ) -> Result<PermissionPosture, ConnectorError>;
    async fn synchronize(
        &self,
        credential: &CredentialHandle<Planning>,
    ) -> Result<ExchangeSnapshot, ConnectorError>;
}
