//! E2E tests for faker-rs integration
//!
//! Tests various faker generators with different data types to ensure
//! the generation produces valid, realistic data.

use chrono::NaiveDate;
use datalchemy_core::{Column, ColumnType, ForeignKey};
use datalchemy_generate::generators::{
    GeneratedValue, GeneratorContext, GeneratorRegistry, RowContext,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::json;

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

/// Tests that all semantic.br.* generators produce valid Brazilian data
#[test]
fn semantic_br_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let br_generators = [
        ("semantic.br.name", "text"),
        ("semantic.br.cpf", "text"),
        ("semantic.br.cnpj", "text"),
        ("semantic.br.cep", "text"),
        ("semantic.br.city", "text"),
        ("semantic.br.uf", "text"),
        ("semantic.br.phone", "text"),
        ("semantic.br.email.safe", "text"),
        ("semantic.br.company.name", "text"),
        ("semantic.br.product.name", "text"),
        ("semantic.br.ip", "text"),
        ("semantic.br.url", "text"),
        ("semantic.br.address", "text"),
        ("semantic.br.rg", "text"),
    ];

    for (gen_id, data_type) in br_generators {
        let column = test_column("test", data_type);
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: Some("pt_BR"),
        };

        let generator = registry
            .generator(gen_id)
            .unwrap_or_else(|| panic!("Generator {} not found", gen_id));

        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(
            result.is_ok(),
            "Generator {} failed: {:?}",
            gen_id,
            result.err()
        );

        let value = result.unwrap();
        assert!(!value.is_null(), "Generator {} produced null value", gen_id);
        eprintln!("  {} => {:?}", gen_id, value);
    }
}

/// Tests that all primitive generators work correctly
#[test]
fn primitive_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    // Test primitive.uuid.v4
    {
        let column = test_column("id", "uuid");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.uuid.v4").unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng).unwrap();
        let uuid_str = result.as_str().unwrap();
        assert_eq!(uuid_str.len(), 36, "UUID should be 36 chars");
        eprintln!("  primitive.uuid.v4 => {:?}", result);
    }

    // Test primitive.int.range
    {
        let column = test_column("age", "int4");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.int.range").unwrap();
        let params = json!({"min": 18, "max": 65});
        let result = generator
            .generate(&mut ctx, Some(&params), &mut rng)
            .unwrap();
        let age = result.as_i64().unwrap();
        assert!((18..=65).contains(&age), "Age should be in range [18, 65]");
        eprintln!("  primitive.int.range(18,65) => {:?}", result);
    }

    // Test primitive.float.range
    {
        let column = test_column("price", "numeric");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.float.range").unwrap();
        let params = json!({"min": 10.0, "max": 1000.0});
        let result = generator
            .generate(&mut ctx, Some(&params), &mut rng)
            .unwrap();
        let price = result.as_f64().unwrap();
        assert!(
            (10.0..=1000.0).contains(&price),
            "Price should be in range [10, 1000]"
        );
        eprintln!("  primitive.float.range(10,1000) => {:?}", result);
    }

    // Test primitive.bool
    {
        let column = test_column("active", "bool");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.bool").unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng).unwrap();
        assert!(
            matches!(result, GeneratedValue::Bool(_)),
            "Should produce a boolean"
        );
        eprintln!("  primitive.bool => {:?}", result);
    }

    // Test primitive.date.range
    {
        let column = test_column("birth_date", "date");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.date.range").unwrap();
        let params = json!({"min": "1990-01-01", "max": "2000-12-31"});
        let result = generator
            .generate(&mut ctx, Some(&params), &mut rng)
            .unwrap();
        let date = result.as_date().unwrap();
        let min_date = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap();
        let max_date = NaiveDate::from_ymd_opt(2000, 12, 31).unwrap();
        assert!(
            date >= min_date && date <= max_date,
            "Date should be in range"
        );
        eprintln!("  primitive.date.range(1990,2000) => {:?}", result);
    }

    // Test primitive.text.pattern
    {
        let column = test_column("status", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator("primitive.text.pattern").unwrap();
        let params = json!({"pattern": "(ativo|inativo|pendente)"});
        let result = generator
            .generate(&mut ctx, Some(&params), &mut rng)
            .unwrap();
        let status = result.as_str().unwrap();
        assert!(
            ["ativo", "inativo", "pendente"].contains(&status),
            "Status should be one of the options"
        );
        eprintln!("  primitive.text.pattern => {:?}", result);
    }
}

/// Tests semantic generators for person-related data
#[test]
fn semantic_person_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let person_generators = [
        "semantic.person.name",
        "semantic.person.first_name",
        "semantic.person.last_name",
        "semantic.person.email",
        "semantic.person.free_email",
        "semantic.person.username",
        "semantic.person.phone",
        "semantic.person.cell",
        "semantic.person.title",
        "semantic.person.suffix",
    ];

    for gen_id in person_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: Some("pt_BR"),
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests semantic generators for company-related data
#[test]
fn semantic_company_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let company_generators = [
        "semantic.company.name",
        "semantic.company.suffix",
        "semantic.company.industry",
        "semantic.company.profession",
        "semantic.company.buzzword",
        "semantic.company.catch_phrase",
        "semantic.company.bs",
        "semantic.company.bs_adj",
        "semantic.company.bs_noun",
        "semantic.company.bs_verb",
    ];

    for gen_id in company_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: Some("pt_BR"),
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests semantic generators for address-related data
#[test]
fn semantic_address_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let address_generators = [
        "semantic.address.city",
        "semantic.address.country",
        "semantic.address.country_code",
        "semantic.address.state",
        "semantic.address.state_abbr",
        "semantic.address.street",
        "semantic.address.street_suffix",
        "semantic.address.building_number",
        "semantic.address.postcode",
        "semantic.address.zipcode",
        "semantic.address.timezone",
        "semantic.address.secondary",
        "semantic.address.secondary_type",
    ];

    for gen_id in address_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: Some("pt_BR"),
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests semantic generators for finance-related data
#[test]
fn semantic_finance_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let finance_generators = [
        "semantic.finance.bic",
        "semantic.finance.isin",
        "semantic.finance.currency_code",
        "semantic.finance.currency_name",
        "semantic.finance.currency_symbol",
    ];

    for gen_id in finance_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests semantic generators for internet-related data
#[test]
fn semantic_internet_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let internet_generators = [
        "semantic.internet.ipv4",
        "semantic.internet.ipv6",
        "semantic.internet.mac",
        "semantic.internet.domain_suffix",
        "semantic.internet.free_email_provider",
        "semantic.internet.user_agent",
        // "semantic.internet.password" requires params (not supported yet)
    ];

    for gen_id in internet_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests time and date semantic generators
#[test]
fn semantic_time_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let time_generators = ["semantic.time.date", "semantic.time.datetime"];

    for gen_id in time_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests color semantic generators
#[test]
fn semantic_color_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let color_generators = [
        "semantic.color.hex",
        "semantic.color.rgb",
        "semantic.color.hsl",
    ];

    for gen_id in color_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests lorem text semantic generators
#[test]
fn semantic_lorem_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let column = test_column("content", "text");
    let mut ctx = GeneratorContext {
        schema: "test",
        table: "test",
        column: &column,
        foreign_keys,
        base_date,
        row_index: 0,
        enum_values: None,
        row: &row,
        foreign: None,
        generator_locale: None,
    };

    let generator = registry.generator("semantic.lorem.word").unwrap();
    let result = generator.generate(&mut ctx, None, &mut rng);
    assert!(result.is_ok(), "Generator semantic.lorem.word failed");
    let word = result.unwrap();
    assert!(
        !word.as_str().unwrap().is_empty(),
        "Word should not be empty"
    );
    eprintln!("  semantic.lorem.word => {:?}", word);
}

/// Tests derive generators (row-based) - check they exist
#[test]
fn derive_generators_exist() {
    let registry = GeneratorRegistry::new();

    // Check that derive generators exist in the registry
    let derive_generators = [
        "derive.email_from_name",
        "derive.end_after_start",
        "derive.fk",
        "derive.money_total",
        "derive.parent_value",
        "derive.updated_after_created",
    ];

    for gen_id in derive_generators {
        let generator = registry.generator(gen_id);
        assert!(
            generator.is_some(),
            "Derive generator {} should exist",
            gen_id
        );
        eprintln!("  {} => exists", gen_id);
    }
}

/// Tests barcode semantic generators
#[test]
fn semantic_barcode_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let barcode_generators = ["semantic.barcode.isbn10", "semantic.barcode.isbn13"];

    for gen_id in barcode_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Tests domain generators (BR specific business data)
#[test]
fn domain_br_money_generator_works() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let column = test_column("valor", "text");
    let mut ctx = GeneratorContext {
        schema: "test",
        table: "test",
        column: &column,
        foreign_keys,
        base_date,
        row_index: 0,
        enum_values: None,
        row: &row,
        foreign: None,
        generator_locale: Some("pt_BR"),
    };

    let generator = registry.generator("semantic.br.money.brl").unwrap();
    let result = generator.generate(&mut ctx, None, &mut rng);
    assert!(
        result.is_ok(),
        "Generator semantic.br.money.brl failed: {:?}",
        result
    );
    eprintln!("  semantic.br.money.brl => {:?}", result.unwrap());
}

/// Tests HTTP status code generator
#[test]
fn semantic_http_status_generator_works() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let column = test_column("status", "text");
    let mut ctx = GeneratorContext {
        schema: "test",
        table: "test",
        column: &column,
        foreign_keys,
        base_date,
        row_index: 0,
        enum_values: None,
        row: &row,
        foreign: None,
        generator_locale: None,
    };

    let generator = registry.generator("semantic.http.status_code").unwrap();
    let result = generator.generate(&mut ctx, None, &mut rng);
    assert!(
        result.is_ok(),
        "Generator semantic.http.status_code failed: {:?}",
        result
    );
    // Status code is returned as text like "200 OK" or just numeric
    let status_str = result.unwrap().as_str().unwrap().to_string();
    assert!(!status_str.is_empty(), "HTTP status should not be empty");
    eprintln!("  semantic.http.status_code => {}", status_str);
}

/// Tests markdown generators
#[test]
fn semantic_markdown_generators_work() {
    let registry = GeneratorRegistry::new();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let row = RowContext::new();
    let foreign_keys: &[ForeignKey] = &[];

    let md_generators = [
        "semantic.markdown.bold",
        "semantic.markdown.italic",
        "semantic.markdown.link",
    ];

    for gen_id in md_generators {
        let column = test_column("test", "text");
        let mut ctx = GeneratorContext {
            schema: "test",
            table: "test",
            column: &column,
            foreign_keys,
            base_date,
            row_index: 0,
            enum_values: None,
            row: &row,
            foreign: None,
            generator_locale: None,
        };

        let generator = registry.generator(gen_id).unwrap();
        let result = generator.generate(&mut ctx, None, &mut rng);
        assert!(result.is_ok(), "Generator {} failed: {:?}", gen_id, result);
        eprintln!("  {} => {:?}", gen_id, result.unwrap());
    }
}

/// Test that generator count matches expected
#[test]
fn generator_count_is_expected() {
    let registry = GeneratorRegistry::new();
    let ids = registry.generator_ids();

    // We expect at least 200+ generators
    assert!(
        ids.len() >= 200,
        "Expected at least 200 generators, found {}",
        ids.len()
    );
    eprintln!("Total generators: {}", ids.len());

    // Check major namespaces exist
    let has_faker = ids.iter().any(|id| id.starts_with("faker."));
    let has_semantic = ids.iter().any(|id| id.starts_with("semantic."));
    let has_primitive = ids.iter().any(|id| id.starts_with("primitive."));
    let has_derive = ids.iter().any(|id| id.starts_with("derive."));
    let has_domain = ids.iter().any(|id| id.starts_with("domain."));

    assert!(has_faker, "Should have faker.* generators");
    assert!(has_semantic, "Should have semantic.* generators");
    assert!(has_primitive, "Should have primitive.* generators");
    assert!(has_derive, "Should have derive.* generators");
    assert!(has_domain, "Should have domain.* generators");
}

// ============================================================================
// CSV Generation Integration Tests
// ============================================================================

use std::fs;
use std::path::PathBuf;

use datalchemy_core::DatabaseSchema;
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_plan::Plan;

fn temp_out_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("datalchemy_faker_e2e");
    let dir = base.join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn load_plan(filename: &str) -> Plan {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plans/examples")
        .join(filename);
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing plan file: {}", path.display()));
    serde_json::from_str(&contents).expect("parse plan JSON")
}

fn load_schema() -> DatabaseSchema {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../datalchemy-introspect/tests/golden/postgres_minimal.schema.json");
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing schema file: {}", path.display()));
    serde_json::from_str(&contents).expect("parse schema JSON")
}

/// Tests full CSV generation with faker_ptbr.plan.json
/// Validates that all faker generators produce valid Brazilian locale data
#[test]
fn csv_generation_with_faker_ptbr_plan() {
    let plan = load_plan("faker_ptbr.plan.json");
    let schema = load_schema();
    let out_dir = temp_out_dir("faker_ptbr");

    let options = GenerateOptions {
        out_dir: out_dir.clone(),
        strict: false,
        ..Default::default()
    };

    let engine = GenerationEngine::new(options);
    let result = engine
        .run(&schema, &plan)
        .expect("run faker_ptbr generation");

    // Verify usuarios.csv was generated with names and emails
    let usuarios_path = result.run_dir.join("crm.usuarios.csv");
    assert!(usuarios_path.exists(), "usuarios.csv should exist");

    let usuarios_csv = fs::read_to_string(&usuarios_path).expect("read usuarios.csv");
    let lines: Vec<&str> = usuarios_csv.lines().collect();

    // Header + 50 data rows
    assert_eq!(
        lines.len(),
        51,
        "usuarios should have 51 lines (header + 50 rows)"
    );

    // Check header contains expected columns
    let header = lines[0];
    assert!(header.contains("nome"), "header should have nome column");
    assert!(header.contains("email"), "header should have email column");

    // Check first data row has valid content
    let row1 = lines[1];
    assert!(!row1.is_empty(), "first row should not be empty");
    eprintln!("usuarios row 1: {}", row1);

    // Verify empresas.csv was generated
    let empresas_path = result.run_dir.join("crm.empresas.csv");
    assert!(empresas_path.exists(), "empresas.csv should exist");

    let empresas_csv = fs::read_to_string(&empresas_path).expect("read empresas.csv");
    let lines: Vec<&str> = empresas_csv.lines().collect();
    assert_eq!(
        lines.len(),
        41,
        "empresas should have 41 lines (header + 40 rows)"
    );
    eprintln!("empresas row 1: {}", lines[1]);

    // Verify produtos.csv was generated
    let produtos_path = result.run_dir.join("crm.produtos.csv");
    assert!(produtos_path.exists(), "produtos.csv should exist");

    let produtos_csv = fs::read_to_string(&produtos_path).expect("read produtos.csv");
    let lines: Vec<&str> = produtos_csv.lines().collect();
    assert_eq!(
        lines.len(),
        61,
        "produtos should have 61 lines (header + 60 rows)"
    );
    eprintln!("produtos row 1: {}", lines[1]);

    // Check report
    assert_eq!(result.report.tables.len(), 6, "should generate 6 tables");
    for table in &result.report.tables {
        assert!(
            table.rows_generated > 0,
            "table {}.{} should have generated rows",
            table.schema,
            table.table
        );
        eprintln!(
            "Generated {}.{}: {} rows",
            table.schema, table.table, table.rows_generated
        );
    }
}

/// Tests full CSV generation with crm_domain.plan.json
/// Validates domain-specific generators (domain.br.*)
#[test]
fn csv_generation_with_crm_domain_plan() {
    let plan = load_plan("crm_domain.plan.json");
    let schema = load_schema();
    let out_dir = temp_out_dir("crm_domain");

    let options = GenerateOptions {
        out_dir: out_dir.clone(),
        strict: false,
        ..Default::default()
    };

    let engine = GenerationEngine::new(options);
    let result = engine
        .run(&schema, &plan)
        .expect("run crm_domain generation");

    // Check that tables were generated
    assert!(
        !result.report.tables.is_empty(),
        "should have generated tables"
    );

    // Verify generation report exists
    let report_path = result.run_dir.join("generation_report.json");
    assert!(report_path.exists(), "generation_report.json should exist");

    let report_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).expect("read report"))
            .expect("parse report JSON");

    // Check that tables were generated (report has tables array with generated rows)
    let tables = report_json.get("tables").and_then(|v| v.as_array());
    assert!(tables.is_some(), "report should have tables array");

    let tables = tables.unwrap();
    assert!(!tables.is_empty(), "tables array should not be empty");

    // Verify all tables have generated rows
    for table in tables {
        let rows = table
            .get("rows_generated")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let name = table.get("table").and_then(|v| v.as_str()).unwrap_or("?");
        assert!(rows > 0, "table {} should have generated rows", name);
    }

    eprintln!("CRM domain generation completed successfully");
    for table in &result.report.tables {
        eprintln!(
            "  {}.{}: {} rows",
            table.schema, table.table, table.rows_generated
        );
    }
}

/// Tests generation is deterministic across runs
#[test]
fn csv_generation_is_deterministic_with_faker() {
    let plan = load_plan("faker_ptbr.plan.json");
    let schema = load_schema();

    let out_dir_a = temp_out_dir("determinism_a");
    let out_dir_b = temp_out_dir("determinism_b");

    // Run generation twice
    let engine_a = GenerationEngine::new(GenerateOptions {
        out_dir: out_dir_a.clone(),
        strict: false,
        ..Default::default()
    });
    let result_a = engine_a.run(&schema, &plan).expect("run A");

    let engine_b = GenerationEngine::new(GenerateOptions {
        out_dir: out_dir_b.clone(),
        strict: false,
        ..Default::default()
    });
    let result_b = engine_b.run(&schema, &plan).expect("run B");

    // Compare CSV outputs
    let csv_a = fs::read_to_string(result_a.run_dir.join("crm.usuarios.csv")).expect("read A");
    let csv_b = fs::read_to_string(result_b.run_dir.join("crm.usuarios.csv")).expect("read B");

    assert_eq!(csv_a, csv_b, "CSV output should be deterministic");
    eprintln!("✓ Determinism verified: faker generates identical output with same seed");
}
