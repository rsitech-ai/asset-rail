use assetrail_lib::security::{CredentialHandle, Execution, Planning};

fn accepts_execution(_: &CredentialHandle<Execution>) {}

fn planning_handle() -> CredentialHandle<Planning> {
    loop {}
}

fn main() {
    let planning = planning_handle();
    accepts_execution(&planning);
}
