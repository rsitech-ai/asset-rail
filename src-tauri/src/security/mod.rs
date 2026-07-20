mod credential;
mod keychain;
mod redaction;
mod vault;

pub use credential::{
    CredentialHandle, CredentialMode, CredentialScope, Execution, Planning, Trading,
};
pub use keychain::MacKeychainVault;
pub use redaction::{RedactedAddress, RedactedCredential, RedactedEvent, RedactedSignature};
pub(crate) use vault::{AuthorizedBinanceQuery, SignBinanceQuery};
pub use vault::{
    CredentialFingerprint, CredentialOperation, CredentialSigningError, CredentialVault,
    MemoryCredentialVault, SecretCredential, VaultError,
};
