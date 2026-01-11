use datalchemy_core::DatabaseSchema;
use schemars::schema_for;
use std::fs;
use std::path::Path;

#[test]
fn json_schema_is_in_sync() {
    let generated = schema_for!(DatabaseSchema);
    let generated_json = serde_json::to_value(&generated).expect("serialize generated schema");

    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/schema.schema.json");
    let stored = fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("missing schema file at {}", schema_path.display()));
    let stored_json: serde_json::Value =
        serde_json::from_str(&stored).expect("parse stored schema");

    assert_eq!(generated_json, stored_json);
}
