use std::fs;
use std::path::PathBuf;

use datalchemy_core::DatabaseSchema;
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_plan::Plan;

fn load_json(path: &PathBuf) -> serde_json::Value {
    let contents =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("missing json at {}", path.display()));
    serde_json::from_str(&contents).expect("parse json")
}

fn load_plan_and_schema() -> (Plan, DatabaseSchema) {
    let plan_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plans/examples/minimal.plan.json");
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../datalchemy-introspect/tests/golden/postgres_minimal.schema.json");

    let plan_json = load_json(&plan_path);
    let schema_json = load_json(&schema_path);

    let plan: Plan = serde_json::from_value(plan_json).expect("parse plan");
    let schema: DatabaseSchema = serde_json::from_value(schema_json).expect("parse schema");

    (plan, schema)
}

#[test]
fn generate_is_deterministic() {
    let (plan, schema) = load_plan_and_schema();

    let out_dir_a = temp_out_dir("run_a");
    let out_dir_b = temp_out_dir("run_b");

    let mut options = GenerateOptions::default();
    options.out_dir = out_dir_a.clone();

    let engine = GenerationEngine::new(options);
    let result_a = engine.run(&schema, &plan).expect("run generation A");

    let mut options = GenerateOptions::default();
    options.out_dir = out_dir_b.clone();

    let engine = GenerationEngine::new(options);
    let result_b = engine.run(&schema, &plan).expect("run generation B");

    let usuarios_a =
        fs::read_to_string(result_a.run_dir.join("crm.usuarios.csv")).expect("read usuarios.csv A");
    let usuarios_b =
        fs::read_to_string(result_b.run_dir.join("crm.usuarios.csv")).expect("read usuarios.csv B");

    assert_eq!(
        usuarios_a, usuarios_b,
        "usuarios.csv should be deterministic"
    );
}

#[test]
fn generate_respects_row_counts() {
    let (plan, schema) = load_plan_and_schema();

    let out_dir = temp_out_dir("run_rows");
    let mut options = GenerateOptions::default();
    options.out_dir = out_dir;

    let engine = GenerationEngine::new(options);
    let result = engine.run(&schema, &plan).expect("run generation");

    let report_path = result.run_dir.join("generation_report.json");
    let report: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&report_path).expect("read generation_report.json"),
    )
    .expect("parse report");

    let tables = report
        .get("tables")
        .and_then(|value| value.as_array())
        .expect("tables array");

    let usuarios_report = tables
        .iter()
        .find(|table| {
            table.get("table") == Some(&serde_json::Value::String("usuarios".to_string()))
        })
        .expect("usuarios report");

    let rows_generated = usuarios_report
        .get("rows_generated")
        .and_then(|value| value.as_u64())
        .expect("rows_generated");

    assert_eq!(rows_generated, 50);
}

fn temp_out_dir(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "datalchemy_generate_{label}_{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&dir).expect("create temp out dir");
    dir
}
