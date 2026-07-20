use assetrail_lib::security::{CredentialVault, MemoryCredentialVault, Planning, SecretCredential};

fn main() {
    let vault = MemoryCredentialVault::default();
    let handle = vault
        .store(SecretCredential::<Planning>::new_binance_hmac(
            "fixture-api-key",
            "fixture-secret",
        ))
        .unwrap();
    let _ = vault.with_credential(
        &handle,
        |credential: &SecretCredential<Planning>| credential.fingerprint(),
    );
}
