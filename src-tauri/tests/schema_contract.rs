use std::{fs, path::Path};

use assetrail_lib::connectors::{
    AuditEventEnvelope, ConnectorCapabilityEnvelope, DestinationProfileEnvelope, ExchangeSnapshot,
    MappingRecord, QuoteEnvelope,
};
use schemars::JsonSchema;

fn assert_schema<Type: JsonSchema>(file_name: &str) {
    let schema = schemars::schema_for!(Type);
    let generated = format!(
        "{}\n",
        serde_json::to_string_pretty(&schema).expect("schema serializes")
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("schemas")
        .join("v1")
        .join(file_name);
    if std::env::var_os("ASSET_RAIL_UPDATE_SCHEMAS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::write(&path, &generated)
            .unwrap_or_else(|error| panic!("cannot update {}: {error}", path.display()));
    }
    let checked_in = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    assert_eq!(checked_in, generated, "schema drift in {}", path.display());
}

#[test]
fn canonical_schema_files_match_rust_contracts() {
    assert_schema::<ConnectorCapabilityEnvelope>("connector-capability.schema.json");
    assert_schema::<ExchangeSnapshot>("exchange-snapshot.schema.json");
    assert_schema::<MappingRecord>("mapping-record.schema.json");
    assert_schema::<DestinationProfileEnvelope>("destination-profile.schema.json");
    assert_schema::<QuoteEnvelope>("quote.schema.json");
    assert_schema::<AuditEventEnvelope>("audit-event.schema.json");
}
