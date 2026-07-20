use std::{collections::HashMap, fmt, marker::PhantomData, sync::Mutex};

use hmac::{Hmac, Mac};
use reqwest::{RequestBuilder, header::HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::credential::{CredentialHandle, CredentialKind, CredentialScope, Planning};
use crate::connectors::{ConnectorId, CredentialScheme, binance::BinanceRequestTarget};

mod operation_sealed {
    pub trait Sealed {}
}

/// A vault-owned credential capability whose implementation is sealed inside
/// the credential-vault subtree.
pub trait CredentialOperation: operation_sealed::Sealed {
    /// The non-secret result released by this operation.
    type Output;

    #[doc(hidden)]
    fn execute(self, credential: &SecretCredential<Planning>) -> Self::Output;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Reads the credential's independently generated, non-secret fingerprint.
pub struct CredentialFingerprint;

impl operation_sealed::Sealed for CredentialFingerprint {}

impl CredentialOperation for CredentialFingerprint {
    type Output = String;

    fn execute(self, credential: &SecretCredential<Planning>) -> Self::Output {
        credential.fingerprint()
    }
}

pub(crate) struct SignBinanceQuery {
    canonical_query: SecretString,
}

impl SignBinanceQuery {
    pub(crate) fn new(canonical_query: String) -> Self {
        Self {
            canonical_query: SecretString::from(canonical_query),
        }
    }
}

impl operation_sealed::Sealed for SignBinanceQuery {}

impl CredentialOperation for SignBinanceQuery {
    type Output = Result<AuthorizedBinanceQuery, CredentialSigningError>;

    fn execute(self, credential: &SecretCredential<Planning>) -> Self::Output {
        if credential.scope.connector_id().as_str() != "binance-spot"
            || credential.scope.scheme() != CredentialScheme::HmacSha256
        {
            return Err(CredentialSigningError::ScopeMismatch);
        }
        let mut api_key = HeaderValue::from_str(credential.api_key.expose_secret())
            .map_err(|_| CredentialSigningError::InvalidApiKey)?;
        if api_key.is_empty() {
            return Err(CredentialSigningError::EmptyApiKey);
        }
        api_key.set_sensitive(true);

        let signature = sign_binance_payload(
            credential.api_secret.expose_secret(),
            self.canonical_query.expose_secret(),
        )?;
        let mut signed_query = self.canonical_query.expose_secret().to_owned();
        signed_query.push_str("&signature=");
        signed_query.push_str(&signature);

        Ok(AuthorizedBinanceQuery {
            api_key,
            signed_query: SecretString::from(signed_query),
        })
    }
}

pub(crate) struct AuthorizedBinanceQuery {
    api_key: HeaderValue,
    signed_query: SecretString,
}

impl AuthorizedBinanceQuery {
    pub(crate) fn apply(self, target: BinanceRequestTarget) -> RequestBuilder {
        target.authorize(self.api_key, self.signed_query)
    }
}

impl fmt::Debug for AuthorizedBinanceQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthorizedBinanceQuery([REDACTED])")
    }
}

fn sign_binance_payload(secret: &str, payload: &str) -> Result<String, CredentialSigningError> {
    if secret.is_empty() {
        return Err(CredentialSigningError::EmptySecret);
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| CredentialSigningError::InvalidSecret)?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CredentialSigningError {
    #[error("the credential scope does not authorize Binance HMAC signing")]
    ScopeMismatch,
    #[error("the Binance API key is empty")]
    EmptyApiKey,
    #[error("the Binance API key is not a valid HTTP header value")]
    InvalidApiKey,
    #[error("the Binance signing secret is empty")]
    EmptySecret,
    #[error("the Binance signing secret is invalid")]
    InvalidSecret,
}

pub trait CredentialVault: Send + Sync {
    /// Stores a planning credential and issues its non-secret opaque handle.
    ///
    /// # Errors
    /// Returns a stable vault error when storage is locked, unavailable, corrupt,
    /// or fails internally.
    fn store(
        &self,
        credential: SecretCredential<Planning>,
    ) -> Result<CredentialHandle<Planning>, VaultError>;

    /// Runs a sealed, security-owned operation while the planning credential is
    /// borrowed from the vault.
    ///
    /// # Errors
    /// Returns a stable vault error when the handle is missing or storage is
    /// locked, unavailable, corrupt, or fails internally.
    fn with_credential<Operation: CredentialOperation>(
        &self,
        handle: &CredentialHandle<Planning>,
        operation: Operation,
    ) -> Result<Operation::Output, VaultError>;

    /// Deletes the planning credential referenced by `handle`.
    ///
    /// # Errors
    /// Returns a stable vault error when the handle is missing or storage is
    /// locked, unavailable, or fails internally.
    fn delete(&self, handle: &CredentialHandle<Planning>) -> Result<(), VaultError>;
}

pub struct SecretCredential<Mode: CredentialKind> {
    scope: CredentialScope,
    api_key: SecretString,
    api_secret: SecretString,
    fingerprint: String,
    mode: PhantomData<fn() -> Mode>,
}

impl SecretCredential<Planning> {
    /// Creates a planning credential bound to one HMAC connector namespace.
    #[must_use]
    pub fn new_hmac(
        connector_id: ConnectorId,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        Self {
            scope: CredentialScope::new(connector_id, CredentialScheme::HmacSha256),
            api_key: SecretString::from(api_key.into()),
            api_secret: SecretString::from(api_secret.into()),
            fingerprint: Uuid::new_v4().simple().to_string()[..16].to_owned(),
            mode: PhantomData,
        }
    }

    /// Creates the currently supported Binance Spot planning credential.
    ///
    /// # Panics
    /// Panics only if the compile-time fixed `binance-spot` connector identifier
    /// stops satisfying [`ConnectorId`] validation.
    #[must_use]
    pub fn new_binance_hmac(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self::new_hmac(
            ConnectorId::parse("binance-spot").expect("fixed Binance connector ID is valid"),
            api_key,
            api_secret,
        )
    }
}

impl<Mode: CredentialKind> SecretCredential<Mode> {
    #[must_use]
    pub const fn scope(&self) -> &CredentialScope {
        &self.scope
    }

    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
}

impl SecretCredential<Planning> {
    const ENCODING_PREFIX: &'static [u8] = b"assetrail-credential-v2\0";

    pub(super) fn encode(&self) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let api_key = self.api_key.expose_secret().as_bytes();
        let api_secret = self.api_secret.expose_secret().as_bytes();
        let connector_id = self.scope.connector_id().as_str().as_bytes();
        let connector_id_length =
            u32::try_from(connector_id.len()).map_err(|_| VaultError::Corrupt)?;
        let api_key_length = u32::try_from(api_key.len()).map_err(|_| VaultError::Corrupt)?;
        let api_secret_length = u32::try_from(api_secret.len()).map_err(|_| VaultError::Corrupt)?;

        let mut encoded = Zeroizing::new(Vec::with_capacity(
            Self::ENCODING_PREFIX.len()
                + 16
                + size_of::<u32>() * 3
                + 1
                + connector_id.len()
                + api_key.len()
                + api_secret.len(),
        ));
        encoded.extend_from_slice(Self::ENCODING_PREFIX);
        encoded.extend_from_slice(self.fingerprint.as_bytes());
        encoded.extend_from_slice(&connector_id_length.to_be_bytes());
        encoded.extend_from_slice(&api_key_length.to_be_bytes());
        encoded.extend_from_slice(&api_secret_length.to_be_bytes());
        encoded.push(credential_scheme_byte(self.scope.scheme())?);
        encoded.extend_from_slice(connector_id);
        encoded.extend_from_slice(api_key);
        encoded.extend_from_slice(api_secret);
        Ok(encoded)
    }

    pub(super) fn decode(encoded: &[u8]) -> Result<Self, VaultError> {
        let fixed_length = Self::ENCODING_PREFIX.len() + 16 + size_of::<u32>() * 3 + 1;
        if encoded.len() < fixed_length || !encoded.starts_with(Self::ENCODING_PREFIX) {
            return Err(VaultError::Corrupt);
        }

        let mut cursor = Self::ENCODING_PREFIX.len();
        let fingerprint =
            std::str::from_utf8(&encoded[cursor..cursor + 16]).map_err(|_| VaultError::Corrupt)?;
        if !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(VaultError::Corrupt);
        }
        cursor += 16;

        let connector_id_length = read_length(encoded, &mut cursor)?;
        let api_key_length = read_length(encoded, &mut cursor)?;
        let api_secret_length = read_length(encoded, &mut cursor)?;
        let scheme = credential_scheme_from_byte(*encoded.get(cursor).ok_or(VaultError::Corrupt)?)?;
        cursor += 1;
        let expected_length = cursor
            .checked_add(connector_id_length)
            .and_then(|length| length.checked_add(api_key_length))
            .and_then(|length| length.checked_add(api_secret_length))
            .ok_or(VaultError::Corrupt)?;
        if expected_length != encoded.len() {
            return Err(VaultError::Corrupt);
        }

        let connector_id_end = cursor + connector_id_length;
        let connector_id = std::str::from_utf8(&encoded[cursor..connector_id_end])
            .map_err(|_| VaultError::Corrupt)?;
        let connector_id = ConnectorId::parse(connector_id).map_err(|_| VaultError::Corrupt)?;
        let api_key_end = connector_id_end + api_key_length;
        let api_key = std::str::from_utf8(&encoded[connector_id_end..api_key_end])
            .map_err(|_| VaultError::Corrupt)?;
        let api_secret =
            std::str::from_utf8(&encoded[api_key_end..]).map_err(|_| VaultError::Corrupt)?;

        Ok(Self {
            scope: CredentialScope::new(connector_id, scheme),
            api_key: SecretString::from(api_key),
            api_secret: SecretString::from(api_secret),
            fingerprint: fingerprint.to_owned(),
            mode: PhantomData,
        })
    }
}

const fn credential_scheme_byte(scheme: CredentialScheme) -> Result<u8, VaultError> {
    match scheme {
        CredentialScheme::HmacSha256 => Ok(1),
        CredentialScheme::Rsa | CredentialScheme::Ed25519 => Err(VaultError::Corrupt),
    }
}

fn credential_scheme_from_byte(value: u8) -> Result<CredentialScheme, VaultError> {
    match value {
        1 => Ok(CredentialScheme::HmacSha256),
        _ => Err(VaultError::Corrupt),
    }
}

fn read_length(encoded: &[u8], cursor: &mut usize) -> Result<usize, VaultError> {
    let end = cursor
        .checked_add(size_of::<u32>())
        .ok_or(VaultError::Corrupt)?;
    let bytes: [u8; 4] = encoded
        .get(*cursor..end)
        .ok_or(VaultError::Corrupt)?
        .try_into()
        .map_err(|_| VaultError::Corrupt)?;
    *cursor = end;
    usize::try_from(u32::from_be_bytes(bytes)).map_err(|_| VaultError::Corrupt)
}

impl<Mode: CredentialKind> fmt::Debug for SecretCredential<Mode> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretCredential([REDACTED])")
    }
}

impl<Mode: CredentialKind> fmt::Display for SecretCredential<Mode> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VaultError {
    #[error("credential vault is locked")]
    Locked,
    #[error("credential was not found")]
    NotFound,
    #[error("credential vault data is corrupt")]
    Corrupt,
    #[error("credential handle scope does not match stored credential")]
    ScopeMismatch,
    #[error("credential vault is unavailable")]
    Unavailable,
    #[error("credential vault failed; diagnostic details were redacted")]
    InternalRedacted,
}

#[derive(Default)]
struct MemoryVaultState {
    credentials: HashMap<String, SecretCredential<Planning>>,
    locked: bool,
}

#[derive(Default)]
pub struct MemoryCredentialVault {
    state: Mutex<MemoryVaultState>,
}

impl MemoryCredentialVault {
    #[must_use]
    pub fn locked() -> Self {
        Self {
            state: Mutex::new(MemoryVaultState {
                credentials: HashMap::new(),
                locked: true,
            }),
        }
    }

    pub fn lock(&self) {
        match self.state.lock() {
            Ok(mut state) => state.locked = true,
            Err(poisoned) => poisoned.into_inner().locked = true,
        }
    }

    pub fn unlock(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.locked = false;
        }
    }
}

impl CredentialVault for MemoryCredentialVault {
    fn store(
        &self,
        credential: SecretCredential<Planning>,
    ) -> Result<CredentialHandle<Planning>, VaultError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| VaultError::InternalRedacted)?;
        if state.locked {
            return Err(VaultError::Locked);
        }
        let account_id = Uuid::new_v4().to_string();
        let scope = credential.scope().clone();
        state.credentials.insert(account_id.clone(), credential);
        Ok(CredentialHandle::from_account_id(account_id, scope))
    }

    fn with_credential<Operation: CredentialOperation>(
        &self,
        handle: &CredentialHandle<Planning>,
        operation: Operation,
    ) -> Result<Operation::Output, VaultError> {
        let state = self
            .state
            .lock()
            .map_err(|_| VaultError::InternalRedacted)?;
        if state.locked {
            return Err(VaultError::Locked);
        }
        let credential = state
            .credentials
            .get(handle.account_id())
            .ok_or(VaultError::NotFound)?;
        if credential.scope() != handle.scope() {
            return Err(VaultError::ScopeMismatch);
        }
        Ok(operation.execute(credential))
    }

    fn delete(&self, handle: &CredentialHandle<Planning>) -> Result<(), VaultError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| VaultError::InternalRedacted)?;
        if state.locked {
            return Err(VaultError::Locked);
        }
        state
            .credentials
            .remove(handle.account_id())
            .map(|_| ())
            .ok_or(VaultError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, mpsc},
        thread,
        time::Duration,
    };

    use super::*;

    #[test]
    fn hmac_signature_matches_the_binance_golden_vector() {
        let payload = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
        let signature = sign_binance_payload(
            "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j",
            payload,
        )
        .expect("published fixture signs");
        assert_eq!(
            signature,
            "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71"
        );
    }

    struct BlockingFingerprint {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }

    impl operation_sealed::Sealed for BlockingFingerprint {}

    impl CredentialOperation for BlockingFingerprint {
        type Output = String;

        fn execute(self, credential: &SecretCredential<Planning>) -> Self::Output {
            self.entered.send(()).expect("operation entry is observed");
            self.release.recv().expect("operation release is delivered");
            credential.fingerprint()
        }
    }

    #[test]
    fn keychain_payload_round_trips_without_formatting_secret_material() {
        let credential = SecretCredential::new_binance_hmac("fixture-api-key", "fixture-secret");
        let expected_fingerprint = credential.fingerprint();
        let encoded = credential.encode().expect("credential is encoded");
        let decoded = SecretCredential::decode(&encoded).expect("credential is decoded");

        assert_eq!(decoded.fingerprint(), expected_fingerprint);
        assert_eq!(decoded.api_key.expose_secret(), "fixture-api-key");
        assert_eq!(decoded.api_secret.expose_secret(), "fixture-secret");
        assert!(!format!("{decoded:?}").contains("fixture-secret"));
    }

    #[test]
    fn malformed_keychain_payload_fails_closed() {
        assert!(matches!(
            SecretCredential::decode(b"fixture-secret"),
            Err(VaultError::Corrupt)
        ));
    }

    #[test]
    fn valid_prefix_and_key_with_malformed_secret_bytes_fails_closed() {
        let credential = SecretCredential::new_binance_hmac("valid-api-key", "valid-secret");
        let mut encoded = credential.encode().expect("credential is encoded");
        *encoded.last_mut().expect("secret byte exists") = 0xff;

        assert!(matches!(
            SecretCredential::decode(&encoded),
            Err(VaultError::Corrupt)
        ));
    }

    #[test]
    fn lock_waits_for_in_flight_operation_and_then_fails_closed() {
        let vault = Arc::new(MemoryCredentialVault::default());
        let handle = vault
            .store(SecretCredential::new_binance_hmac(
                "fixture-api-key",
                "fixture-secret",
            ))
            .expect("credential is stored");
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let operation_vault = Arc::clone(&vault);
        let operation_handle = handle.clone();
        let operation_thread = thread::spawn(move || {
            operation_vault.with_credential(
                &operation_handle,
                BlockingFingerprint {
                    entered: entered_tx,
                    release: release_rx,
                },
            )
        });
        entered_rx.recv().expect("operation holds the vault state");

        let (lock_started_tx, lock_started_rx) = mpsc::channel();
        let (lock_returned_tx, lock_returned_rx) = mpsc::channel();
        let locking_vault = Arc::clone(&vault);
        let lock_thread = thread::spawn(move || {
            lock_started_tx.send(()).expect("lock attempt is observed");
            locking_vault.lock();
            lock_returned_tx.send(()).expect("lock return is observed");
        });
        lock_started_rx.recv().expect("lock attempt starts");
        assert!(
            lock_returned_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "lock returned while a credential operation was still in flight"
        );

        release_tx.send(()).expect("operation is released");
        operation_thread
            .join()
            .expect("operation thread does not panic")
            .expect("in-flight operation completes");
        lock_returned_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("lock returns after the operation completes");
        lock_thread.join().expect("lock thread does not panic");

        assert!(matches!(
            vault.with_credential(&handle, CredentialFingerprint),
            Err(VaultError::Locked)
        ));
    }
}
