use chrono::NaiveDate;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::json;

use datalchemy_core::{Column, ColumnType, ForeignKey};
use datalchemy_generate::errors::GenerationError;
use datalchemy_generate::faker_rs::FakeRsAdapter;
use datalchemy_generate::generators::{GeneratorContext, GeneratorRegistry, RowContext};

fn test_column(name: &str, data_type: &str) -> Column {
    Column {
        ordinal_position: 1,
        name: name.to_string(),
        column_type: ColumnType {
            data_type: data_type.to_string(),
            udt_schema: "pg_catalog".to_string(),
            udt_name: data_type.to_string(),
            character_max_length: None,
            numeric_precision: None,
            numeric_scale: None,
            collation: None,
        },
        is_nullable: false,
        default: None,
        identity: None,
        generated: None,
        comment: None,
    }
}

#[test]
fn generator_ids_are_sorted_and_unique() {
    let registry = GeneratorRegistry::new();
    let ids = registry.generator_ids();
    assert!(!ids.is_empty());

    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids, sorted);
}

#[test]
fn faker_unknown_id_errors() {
    let result = FakeRsAdapter::validate("faker.unknown.id", None, None);
    assert!(matches!(result, Err(GenerationError::InvalidPlan(_))));
}

#[test]
fn faker_rejects_unknown_params() {
    let registry = GeneratorRegistry::new();
    let generator = registry
        .generator("faker.name.raw.Name")
        .expect("faker generator exists");
    let column = test_column("nome", "text");
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];
    let mut ctx = GeneratorContext {
        schema: "crm",
        table: "usuarios",
        column: &column,
        foreign_keys,
        base_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap_or_default(),
        row_index: 0,
        enum_values: None,
        row: &row,
        foreign: None,
        generator_locale: Some("pt_BR"),
    };
    let params = json!({"min": 1});
    let mut rng = ChaCha8Rng::seed_from_u64(1);

    let result = generator.generate(&mut ctx, Some(&params), &mut rng);
    assert!(matches!(result, Err(GenerationError::InvalidPlan(_))));
}
