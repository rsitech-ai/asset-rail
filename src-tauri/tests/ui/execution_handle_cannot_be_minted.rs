use assetrail_lib::security::{CredentialHandle, Execution};

fn main() {
    let _ = CredentialHandle::<Execution>::from_account_id("execution".to_owned());
}
