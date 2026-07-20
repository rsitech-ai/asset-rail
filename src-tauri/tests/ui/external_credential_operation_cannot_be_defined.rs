use assetrail_lib::security::{CredentialOperation, Planning, SecretCredential};

struct ExternalOperation;

impl CredentialOperation for ExternalOperation {
    type Output = String;

    fn execute(self, credential: &SecretCredential<Planning>) -> Self::Output {
        credential.fingerprint()
    }
}

fn main() {}
