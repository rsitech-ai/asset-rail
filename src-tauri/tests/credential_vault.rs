use assetrail_lib::connectors::CredentialScheme;
use assetrail_lib::security::{
    CredentialFingerprint, CredentialVault, MacKeychainVault, MemoryCredentialVault, Planning,
    SecretCredential, VaultError,
};

fn fixture_planning_credential() -> SecretCredential<Planning> {
    SecretCredential::new_binance_hmac("fixture-api-key", "fixture-secret")
}

#[test]
fn stored_secret_can_only_be_used_by_a_security_owned_operation() {
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(fixture_planning_credential())
        .expect("fixture credential is stored");

    assert_eq!(handle.scope().connector_id().as_str(), "binance-spot");
    assert_eq!(handle.scope().scheme(), CredentialScheme::HmacSha256);

    let fingerprint = vault
        .with_credential(&handle, CredentialFingerprint)
        .expect("stored credential is available to the fingerprint operation");

    assert_eq!(fingerprint.len(), 16);
    assert!(!format!("{handle:?}").contains("fixture-secret"));
}

#[test]
fn locked_vault_fails_closed_before_storing_plaintext() {
    let vault = MemoryCredentialVault::locked();

    assert!(matches!(
        vault.store(fixture_planning_credential()),
        Err(VaultError::Locked)
    ));
}

#[test]
fn existing_credentials_are_inaccessible_while_the_vault_is_locked() {
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(fixture_planning_credential())
        .expect("fixture credential is stored");

    vault.lock();

    assert!(matches!(
        vault.with_credential(&handle, CredentialFingerprint),
        Err(VaultError::Locked)
    ));
    assert_eq!(vault.delete(&handle), Err(VaultError::Locked));

    vault.unlock();
    assert!(
        vault
            .with_credential(&handle, CredentialFingerprint)
            .is_ok()
    );
}

#[test]
fn deleting_a_credential_invalidates_its_handle() {
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(fixture_planning_credential())
        .expect("fixture credential is stored");

    vault.delete(&handle).expect("stored credential is deleted");

    assert!(matches!(
        vault.with_credential(&handle, CredentialFingerprint),
        Err(VaultError::NotFound)
    ));
    assert_eq!(vault.delete(&handle), Err(VaultError::NotFound));
}

#[test]
fn duplicate_credentials_remain_isolated_by_non_secret_handles() {
    let vault = MemoryCredentialVault::default();
    let first = vault
        .store(fixture_planning_credential())
        .expect("first fixture credential is stored");
    let second = vault
        .store(fixture_planning_credential())
        .expect("duplicate fixture credential is stored independently");

    vault.delete(&first).expect("first credential is deleted");

    assert!(matches!(
        vault.with_credential(&first, CredentialFingerprint),
        Err(VaultError::NotFound)
    ));
    assert!(
        vault
            .with_credential(&second, CredentialFingerprint)
            .is_ok()
    );
}

#[test]
fn deleted_credential_can_be_replaced_without_reviving_the_old_handle() {
    let vault = MemoryCredentialVault::default();
    let old = vault
        .store(fixture_planning_credential())
        .expect("old credential is stored");
    vault.delete(&old).expect("old credential is deleted");

    let replacement = vault
        .store(SecretCredential::new_binance_hmac(
            "replacement-api-key",
            "replacement-secret",
        ))
        .expect("replacement credential is stored");

    assert!(matches!(
        vault.with_credential(&old, CredentialFingerprint),
        Err(VaultError::NotFound)
    ));
    let replacement_fingerprint = vault
        .with_credential(&replacement, CredentialFingerprint)
        .expect("replacement is available to the fingerprint operation");
    assert_eq!(replacement_fingerprint.len(), 16);
}

#[test]
fn credential_and_handle_formatting_never_contains_secret_material() {
    let credential = fixture_planning_credential();
    for rendered in [format!("{credential}"), format!("{credential:?}")] {
        assert!(rendered.contains("REDACTED"));
        assert!(!rendered.contains("fixture-api-key"));
        assert!(!rendered.contains("fixture-secret"));
    }

    let vault = MemoryCredentialVault::default();
    let handle = vault.store(credential).expect("credential is stored");
    let rendered_handle = format!("{handle:?}");
    assert!(!rendered_handle.contains("fixture-api-key"));
    assert!(!rendered_handle.contains("fixture-secret"));
}

#[test]
fn mac_keychain_adapter_is_a_thread_safe_vault_with_a_fixed_service() {
    fn assert_vault<T: CredentialVault + Default + Send + Sync>() {}

    assert_vault::<MacKeychainVault>();
    assert_eq!(
        MacKeychainVault::SERVICE,
        "ai.rsitech.assetrail.credentials"
    );
}

#[test]
fn raw_credential_parts_cannot_escape_the_public_vault_boundary() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/credential_plaintext_cannot_escape.rs");
    cases.compile_fail("tests/ui/arbitrary_credential_closure_cannot_be_used.rs");
    cases.compile_fail("tests/ui/external_credential_operation_cannot_be_defined.rs");
    cases.compile_fail("tests/ui/execution_handle_cannot_be_minted.rs");
    cases.compile_fail("tests/ui/binance_authorization_helpers_are_not_public.rs");
    cases.compile_fail("tests/ui/operator_refresh_permit_cannot_be_forged.rs");
}
