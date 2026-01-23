use datalchemy_core::DatabaseSchema;
use datalchemy_plan::{validate_plan, validate_plan_json};
use std::fs;
use std::path::Path;

fn load_json(path: &Path) -> serde_json::Value {
    let contents =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("missing json at {}", path.display()));
    serde_json::from_str(&contents).expect("parse json")
}

#[test]
fn minimal_plan_validates_against_schema() {
    let plan_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plans/examples/minimal.plan.json");
    let plan_schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/plan.schema.json");
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../datalchemy-introspect/tests/golden/postgres_minimal.schema.json");

    let plan_json = load_json(&plan_path);
    let plan_schema_json = load_json(&plan_schema_path);
    let schema_json = load_json(&schema_path);
    let schema: DatabaseSchema = serde_json::from_value(schema_json).expect("parse schema.json");

    let structural =
        validate_plan_json(&plan_json, &plan_schema_json).expect("validate plan json schema");
    assert!(structural.errors.is_empty(), "structural errors found");

    let validated = validate_plan(&plan_json, &plan_schema_json, &schema)
        .expect("plan validation should succeed");
    assert!(validated.warnings.is_empty(), "unexpected warnings");
}
