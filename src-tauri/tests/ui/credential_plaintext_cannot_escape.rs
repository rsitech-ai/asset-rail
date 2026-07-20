use assetrail_lib::security::{Planning, SecretCredential};

fn main() {
    let credential =
        SecretCredential::<Planning>::new_binance_hmac("fixture-api-key", "fixture-secret");
    let _ = credential.with_secret_parts(|_, api_secret| api_secret.to_owned());
}
