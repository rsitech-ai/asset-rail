use std::sync::Arc;

use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    CredentialHandle, CredentialOperation, CredentialVault, Planning, SecretCredential, VaultError,
};

const ERR_SEC_NOT_AVAILABLE: i32 = -25_291;
const ERR_SEC_AUTH_FAILED: i32 = -25_293;
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25_300;
const ERR_SEC_INTERACTION_NOT_ALLOWED: i32 = -25_308;

pub struct MacKeychainVault {
    backend: Arc<dyn KeychainBackend>,
}

impl MacKeychainVault {
    pub const SERVICE: &'static str = "ai.rsitech.assetrail.credentials";

    #[must_use]
    pub fn new() -> Self {
        Self {
            backend: Arc::new(ProductionKeychainBackend),
        }
    }

    #[cfg(test)]
    fn with_backend(backend: Arc<dyn KeychainBackend>) -> Self {
        Self { backend }
    }

    fn store_with_account_id(
        &self,
        account_id: String,
        credential: &SecretCredential<Planning>,
    ) -> Result<CredentialHandle<Planning>, VaultError> {
        let encoded = credential.encode()?;
        self.backend
            .add_or_replace(&store_request(&account_id), &encoded)
            .map_err(VaultError::from)?;
        Ok(CredentialHandle::from_account_id(
            account_id,
            credential.scope().clone(),
        ))
    }
}

impl Default for MacKeychainVault {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialVault for MacKeychainVault {
    fn store(
        &self,
        credential: SecretCredential<Planning>,
    ) -> Result<CredentialHandle<Planning>, VaultError> {
        self.store_with_account_id(Uuid::new_v4().to_string(), &credential)
    }

    fn with_credential<Operation: CredentialOperation>(
        &self,
        handle: &CredentialHandle<Planning>,
        operation: Operation,
    ) -> Result<Operation::Output, VaultError> {
        let encoded = Zeroizing::new(
            self.backend
                .load(&lookup_request(handle.account_id()))
                .map_err(VaultError::from)?,
        );
        let credential = SecretCredential::decode(&encoded)?;
        if credential.scope() != handle.scope() {
            return Err(VaultError::ScopeMismatch);
        }
        Ok(operation.execute(&credential))
    }

    fn delete(&self, handle: &CredentialHandle<Planning>) -> Result<(), VaultError> {
        self.backend
            .delete(&lookup_request(handle.account_id()))
            .map_err(VaultError::from)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeychainProtection {
    AccessibleWhenUnlockedThisDeviceOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KeychainLookup<'a> {
    service: &'static str,
    account: &'a str,
    use_protected_keychain: bool,
    synchronizing: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KeychainStore<'a> {
    lookup: KeychainLookup<'a>,
    protection: KeychainProtection,
}

fn lookup_request(account: &str) -> KeychainLookup<'_> {
    KeychainLookup {
        service: MacKeychainVault::SERVICE,
        account,
        use_protected_keychain: true,
        synchronizing: false,
    }
}

fn store_request(account: &str) -> KeychainStore<'_> {
    KeychainStore {
        lookup: lookup_request(account),
        protection: KeychainProtection::AccessibleWhenUnlockedThisDeviceOnly,
    }
}

trait KeychainBackend: Send + Sync {
    fn add_or_replace(
        &self,
        request: &KeychainStore<'_>,
        password: &[u8],
    ) -> Result<(), KeychainBackendError>;

    fn load(&self, request: &KeychainLookup<'_>) -> Result<Vec<u8>, KeychainBackendError>;

    fn delete(&self, request: &KeychainLookup<'_>) -> Result<(), KeychainBackendError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeychainBackendError {
    Locked,
    NotFound,
    Unavailable,
    InternalRedacted,
}

impl From<KeychainBackendError> for VaultError {
    fn from(error: KeychainBackendError) -> Self {
        match error {
            KeychainBackendError::Locked => Self::Locked,
            KeychainBackendError::NotFound => Self::NotFound,
            KeychainBackendError::Unavailable => Self::Unavailable,
            KeychainBackendError::InternalRedacted => Self::InternalRedacted,
        }
    }
}

fn classify_keychain_status(status: i32) -> KeychainBackendError {
    match status {
        ERR_SEC_NOT_AVAILABLE => KeychainBackendError::Unavailable,
        ERR_SEC_AUTH_FAILED | ERR_SEC_INTERACTION_NOT_ALLOWED => KeychainBackendError::Locked,
        ERR_SEC_ITEM_NOT_FOUND => KeychainBackendError::NotFound,
        _ => KeychainBackendError::InternalRedacted,
    }
}

struct ProductionKeychainBackend;

#[cfg(target_os = "macos")]
impl KeychainBackend for ProductionKeychainBackend {
    fn add_or_replace(
        &self,
        request: &KeychainStore<'_>,
        password: &[u8],
    ) -> Result<(), KeychainBackendError> {
        use security_framework::{
            access_control::{ProtectionMode, SecAccessControl},
            passwords::set_generic_password_options,
        };

        let mut options = password_options(&request.lookup);
        let protection = match request.protection {
            KeychainProtection::AccessibleWhenUnlockedThisDeviceOnly => {
                ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly
            }
        };
        let access_control = SecAccessControl::create_with_protection(Some(protection), 0)
            .map_err(map_keychain_error)?;
        options.set_access_control(access_control);
        set_generic_password_options(password, options).map_err(map_keychain_error)
    }

    fn load(&self, request: &KeychainLookup<'_>) -> Result<Vec<u8>, KeychainBackendError> {
        security_framework::passwords::generic_password(password_options(request))
            .map_err(map_keychain_error)
    }

    fn delete(&self, request: &KeychainLookup<'_>) -> Result<(), KeychainBackendError> {
        security_framework::passwords::delete_generic_password_options(password_options(request))
            .map_err(map_keychain_error)
    }
}

#[cfg(target_os = "macos")]
fn password_options(
    request: &KeychainLookup<'_>,
) -> security_framework::passwords::PasswordOptions {
    let mut options = security_framework::passwords::PasswordOptions::new_generic_password(
        request.service,
        request.account,
    );
    if request.use_protected_keychain {
        options.use_protected_keychain();
    }
    options.set_access_synchronized(Some(request.synchronizing));
    options
}

#[cfg(target_os = "macos")]
fn map_keychain_error(error: security_framework::base::Error) -> KeychainBackendError {
    classify_keychain_status(error.code())
}

#[cfg(not(target_os = "macos"))]
impl KeychainBackend for ProductionKeychainBackend {
    fn add_or_replace(
        &self,
        _request: &KeychainStore<'_>,
        _password: &[u8],
    ) -> Result<(), KeychainBackendError> {
        Err(KeychainBackendError::Unavailable)
    }

    fn load(&self, _request: &KeychainLookup<'_>) -> Result<Vec<u8>, KeychainBackendError> {
        Err(KeychainBackendError::Unavailable)
    }

    fn delete(&self, _request: &KeychainLookup<'_>) -> Result<(), KeychainBackendError> {
        Err(KeychainBackendError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;
    use crate::security::CredentialFingerprint;

    type ItemKey = (String, String);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedLookup {
        service: String,
        account: String,
        use_protected_keychain: bool,
        synchronizing: bool,
    }

    impl From<&KeychainLookup<'_>> for RecordedLookup {
        fn from(request: &KeychainLookup<'_>) -> Self {
            Self {
                service: request.service.to_owned(),
                account: request.account.to_owned(),
                use_protected_keychain: request.use_protected_keychain,
                synchronizing: request.synchronizing,
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum RecordedOperation {
        AddOrReplace {
            lookup: RecordedLookup,
            protection: KeychainProtection,
        },
        Load(RecordedLookup),
        Delete(RecordedLookup),
    }

    #[derive(Default)]
    struct FakeState {
        items: HashMap<ItemKey, Zeroizing<Vec<u8>>>,
        operations: Vec<RecordedOperation>,
        next_error: Option<KeychainBackendError>,
    }

    #[derive(Clone, Default)]
    struct FakeKeychainBackend {
        state: Arc<Mutex<FakeState>>,
    }

    impl FakeKeychainBackend {
        fn fail_next(&self, error: KeychainBackendError) {
            self.state.lock().expect("fake state lock").next_error = Some(error);
        }

        fn operations(&self) -> Vec<RecordedOperation> {
            self.state
                .lock()
                .expect("fake state lock")
                .operations
                .clone()
        }

        fn take_error(state: &mut FakeState) -> Result<(), KeychainBackendError> {
            match state.next_error.take() {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }

    impl KeychainBackend for FakeKeychainBackend {
        fn add_or_replace(
            &self,
            request: &KeychainStore<'_>,
            password: &[u8],
        ) -> Result<(), KeychainBackendError> {
            let mut state = self.state.lock().expect("fake state lock");
            state.operations.push(RecordedOperation::AddOrReplace {
                lookup: RecordedLookup::from(&request.lookup),
                protection: request.protection,
            });
            Self::take_error(&mut state)?;
            state.items.insert(
                (
                    request.lookup.service.to_owned(),
                    request.lookup.account.to_owned(),
                ),
                Zeroizing::new(password.to_vec()),
            );
            Ok(())
        }

        fn load(&self, request: &KeychainLookup<'_>) -> Result<Vec<u8>, KeychainBackendError> {
            let mut state = self.state.lock().expect("fake state lock");
            state
                .operations
                .push(RecordedOperation::Load(RecordedLookup::from(request)));
            Self::take_error(&mut state)?;
            state
                .items
                .get(&(request.service.to_owned(), request.account.to_owned()))
                .map(|password| password.to_vec())
                .ok_or(KeychainBackendError::NotFound)
        }

        fn delete(&self, request: &KeychainLookup<'_>) -> Result<(), KeychainBackendError> {
            let mut state = self.state.lock().expect("fake state lock");
            state
                .operations
                .push(RecordedOperation::Delete(RecordedLookup::from(request)));
            Self::take_error(&mut state)?;
            state
                .items
                .remove(&(request.service.to_owned(), request.account.to_owned()))
                .map(|_| ())
                .ok_or(KeychainBackendError::NotFound)
        }
    }

    fn fake_vault() -> (MacKeychainVault, FakeKeychainBackend) {
        let backend = FakeKeychainBackend::default();
        let vault = MacKeychainVault::with_backend(Arc::new(backend.clone()));
        (vault, backend)
    }

    #[test]
    fn injected_backend_adds_loads_and_deletes_without_live_keychain_access() {
        let (vault, _) = fake_vault();
        let credential = SecretCredential::new_binance_hmac("fixture-key", "fixture-secret");
        let expected_fingerprint = credential.fingerprint();
        let handle = vault.store(credential).expect("credential is added");

        let fingerprint = vault
            .with_credential(&handle, CredentialFingerprint)
            .expect("credential is loaded");
        assert_eq!(fingerprint, expected_fingerprint);

        vault.delete(&handle).expect("credential is deleted");
        assert!(matches!(
            vault.with_credential(&handle, CredentialFingerprint),
            Err(VaultError::NotFound)
        ));
    }

    #[test]
    fn duplicate_accounts_are_isolated_and_same_account_is_replaced() {
        let (vault, _) = fake_vault();
        let first = vault
            .store(SecretCredential::new_binance_hmac(
                "same-key",
                "first-secret",
            ))
            .expect("first credential is stored");
        let second = vault
            .store(SecretCredential::new_binance_hmac(
                "same-key",
                "second-secret",
            ))
            .expect("second credential is stored");
        assert_ne!(first.account_id(), second.account_id());
        vault.delete(&first).expect("first credential is deleted");
        assert!(
            vault
                .with_credential(&second, CredentialFingerprint)
                .is_ok()
        );

        let fixed_account = "00000000-0000-4000-8000-000000000001".to_owned();
        let original = vault
            .store_with_account_id(
                fixed_account.clone(),
                &SecretCredential::new_binance_hmac("fixed-key", "old-secret"),
            )
            .expect("fixed account is stored");
        let replacement_credential = SecretCredential::new_binance_hmac("fixed-key", "new-secret");
        let replacement_fingerprint = replacement_credential.fingerprint();
        let replacement = vault
            .store_with_account_id(fixed_account, &replacement_credential)
            .expect("fixed account is replaced");
        assert_eq!(original.account_id(), replacement.account_id());
        let replaced_fingerprint = vault
            .with_credential(&replacement, CredentialFingerprint)
            .expect("replacement is loaded");
        assert_eq!(replaced_fingerprint, replacement_fingerprint);
    }

    #[test]
    fn backend_receives_fixed_service_account_and_device_only_options() {
        let (vault, backend) = fake_vault();
        let handle = vault
            .store(SecretCredential::new_binance_hmac(
                "fixture-key",
                "fixture-secret",
            ))
            .expect("credential is stored");
        vault
            .with_credential(&handle, CredentialFingerprint)
            .expect("credential is loaded");
        vault.delete(&handle).expect("credential is deleted");

        let expected_lookup = RecordedLookup {
            service: MacKeychainVault::SERVICE.to_owned(),
            account: handle.account_id().to_owned(),
            use_protected_keychain: true,
            synchronizing: false,
        };
        assert!(Uuid::parse_str(&expected_lookup.account).is_ok());
        assert_eq!(
            backend.operations(),
            vec![
                RecordedOperation::AddOrReplace {
                    lookup: expected_lookup.clone(),
                    protection: KeychainProtection::AccessibleWhenUnlockedThisDeviceOnly,
                },
                RecordedOperation::Load(expected_lookup.clone()),
                RecordedOperation::Delete(expected_lookup),
            ]
        );
    }

    #[test]
    fn injected_backend_errors_map_to_stable_vault_categories() {
        let (vault, backend) = fake_vault();
        let handle = vault
            .store(SecretCredential::new_binance_hmac(
                "stored-key",
                "stored-secret",
            ))
            .expect("credential is stored before injected failures");

        backend.fail_next(KeychainBackendError::Locked);
        assert!(matches!(
            vault.with_credential(&handle, CredentialFingerprint),
            Err(VaultError::Locked)
        ));

        backend.fail_next(KeychainBackendError::Unavailable);
        assert_eq!(vault.delete(&handle), Err(VaultError::Unavailable));

        backend.fail_next(KeychainBackendError::InternalRedacted);
        assert!(matches!(
            vault.store(SecretCredential::new_binance_hmac("key", "secret")),
            Err(VaultError::InternalRedacted)
        ));
    }

    #[test]
    fn security_framework_statuses_distinguish_unavailable_from_locked() {
        assert_eq!(
            classify_keychain_status(ERR_SEC_NOT_AVAILABLE),
            KeychainBackendError::Unavailable
        );
        assert_eq!(
            classify_keychain_status(ERR_SEC_AUTH_FAILED),
            KeychainBackendError::Locked
        );
        assert_eq!(
            classify_keychain_status(ERR_SEC_INTERACTION_NOT_ALLOWED),
            KeychainBackendError::Locked
        );
        assert_eq!(
            classify_keychain_status(ERR_SEC_ITEM_NOT_FOUND),
            KeychainBackendError::NotFound
        );
        assert_eq!(
            classify_keychain_status(-1),
            KeychainBackendError::InternalRedacted
        );
    }
}
