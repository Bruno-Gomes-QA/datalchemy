use chrono::NaiveDate;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::json;

use datalchemy_core::{Column, ColumnType};
use datalchemy_generate::errors::GenerationError;
use datalchemy_generate::generators::{
    GeneratedValue, GeneratorContext, GeneratorRegistry, TransformContext,
};

fn test_column(name: &str, data_type: &str, is_nullable: bool) -> Column {
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
        is_nullable,
        default: None,
        identity: None,
        generated: None,
        comment: None,
    }
}

#[test]
fn primitive_int_range_rejects_invalid_bounds() {
    let registry = GeneratorRegistry::new();
    let generator = registry
        .generator("primitive.int.range")
        .expect("generator exists");
    let column = test_column("quantidade", "integer", false);
    let ctx = GeneratorContext {
        schema: "crm",
        table: "itens_cotacao",
        column: &column,
        base_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap_or_else(NaiveDate::default),
        row_index: 0,
        enum_values: None,
    };
    let params = json!({"min": 10, "max": 1});
    let mut rng = ChaCha8Rng::seed_from_u64(1);

    let result = generator.generate(&ctx, Some(&params), &mut rng);
    assert!(matches!(result, Err(GenerationError::InvalidPlan(_))));
}

#[test]
fn null_rate_transform_rejects_not_null_column() {
    let registry = GeneratorRegistry::new();
    let transform = registry
        .transform("transform.null_rate")
        .expect("transform exists");
    let column = test_column("email", "text", false);
    let ctx = TransformContext {
        schema: "crm",
        table: "usuarios",
        column: &column,
        base_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap_or_else(NaiveDate::default),
        row_index: 0,
        strict: false,
    };
    let params = json!({"rate": 0.5});
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let result = transform.apply(
        GeneratedValue::Text("teste".to_string()),
        &ctx,
        Some(&params),
        &mut rng,
    );
    assert!(matches!(result, Err(GenerationError::InvalidPlan(_))));
}
