use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use datalchemy_core::{
    Column, ColumnType, Constraint, DatabaseSchema, FkAction, FkMatchType, ForeignKey, PrimaryKey,
    Schema, Table, TableKind,
};
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_plan::{ColumnGeneratorRule, Plan, PlanOptions, Rule, SchemaRef, Target};

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn column(
    ordinal: i16,
    name: &str,
    data_type: &str,
    udt_name: &str,
    numeric_scale: Option<i32>,
) -> Column {
    Column {
        ordinal_position: ordinal,
        name: name.to_string(),
        column_type: ColumnType {
            data_type: data_type.to_string(),
            udt_schema: "pg_catalog".to_string(),
            udt_name: udt_name.to_string(),
            character_max_length: None,
            numeric_precision: None,
            numeric_scale,
            collation: None,
        },
        is_nullable: false,
        default: None,
        identity: None,
        generated: None,
        comment: None,
    }
}

fn schema_fixture() -> DatabaseSchema {
    let users = Table {
        name: "users".to_string(),
        kind: TableKind::Table,
        comment: None,
        columns: vec![
            column(1, "id", "uuid", "uuid", None),
            column(2, "name", "text", "text", None),
            column(3, "email", "text", "text", None),
            column(
                4,
                "created_at",
                "timestamp without time zone",
                "timestamp",
                None,
            ),
        ],
        constraints: vec![Constraint::PrimaryKey(PrimaryKey {
            name: None,
            columns: vec!["id".to_string()],
        })],
        indexes: Vec::new(),
    };

    let orders = Table {
        name: "orders".to_string(),
        kind: TableKind::Table,
        comment: None,
        columns: vec![
            column(1, "id", "uuid", "uuid", None),
            column(2, "user_id", "uuid", "uuid", None),
            column(3, "user_email", "text", "text", None),
            column(4, "price", "numeric", "numeric", Some(2)),
            column(5, "qty", "integer", "int4", None),
            column(6, "discount", "numeric", "numeric", Some(2)),
            column(7, "total", "numeric", "numeric", Some(2)),
            column(
                8,
                "created_at",
                "timestamp without time zone",
                "timestamp",
                None,
            ),
            column(
                9,
                "updated_at",
                "timestamp without time zone",
                "timestamp",
                None,
            ),
        ],
        constraints: vec![
            Constraint::PrimaryKey(PrimaryKey {
                name: None,
                columns: vec!["id".to_string()],
            }),
            Constraint::ForeignKey(ForeignKey {
                name: None,
                columns: vec!["user_id".to_string()],
                referenced_schema: "public".to_string(),
                referenced_table: "users".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_update: FkAction::NoAction,
                on_delete: FkAction::NoAction,
                match_type: FkMatchType::Simple,
                is_deferrable: false,
                initially_deferred: false,
            }),
        ],
        indexes: Vec::new(),
    };

    DatabaseSchema {
        schema_version: "0.2".to_string(),
        engine: "postgres".to_string(),
        database: None,
        schemas: vec![Schema {
            name: "public".to_string(),
            tables: vec![users, orders],
        }],
        enums: Vec::new(),
        schema_fingerprint: None,
    }
}

fn plan_fixture() -> Plan {
    let mut rules = Vec::new();

    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "id".to_string(),
        generator: "primitive.uuid.v4".to_string(),
        params: None,
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "name".to_string(),
        generator: "primitive.text.pattern".to_string(),
        params: Some(serde_json::json!({"pattern": "User-####"})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "email".to_string(),
        generator: "derive.email_from_name".to_string(),
        params: Some(serde_json::json!({"input_columns": ["name"], "domain": "example.com"})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "users".to_string(),
        column: "created_at".to_string(),
        generator: "primitive.timestamp.range".to_string(),
        params: Some(
            serde_json::json!({"min": "2024-01-01T00:00:00", "max": "2024-01-10T23:59:59"}),
        ),
        transforms: Vec::new(),
    }));

    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "id".to_string(),
        generator: "primitive.uuid.v4".to_string(),
        params: None,
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "user_id".to_string(),
        generator: "derive.fk".to_string(),
        params: None,
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "user_email".to_string(),
        generator: "derive.parent_value".to_string(),
        params: Some(serde_json::json!({
            "input_columns": ["user_id"],
            "parent_schema": "public",
            "parent_table": "users",
            "parent_column": "email"
        })),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "price".to_string(),
        generator: "primitive.float.range".to_string(),
        params: Some(serde_json::json!({"min": 10.0, "max": 120.0})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "qty".to_string(),
        generator: "primitive.int.range".to_string(),
        params: Some(serde_json::json!({"min": 1, "max": 5})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "discount".to_string(),
        generator: "primitive.float.range".to_string(),
        params: Some(serde_json::json!({"min": 0.0, "max": 5.0})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "total".to_string(),
        generator: "derive.money_total".to_string(),
        params: Some(serde_json::json!({"input_columns": ["price", "qty", "discount"]})),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "created_at".to_string(),
        generator: "primitive.timestamp.range".to_string(),
        params: Some(
            serde_json::json!({"min": "2024-01-01T00:00:00", "max": "2024-01-10T23:59:59"}),
        ),
        transforms: Vec::new(),
    }));
    rules.push(Rule::ColumnGenerator(ColumnGeneratorRule {
        schema: "public".to_string(),
        table: "orders".to_string(),
        column: "updated_at".to_string(),
        generator: "derive.updated_after_created".to_string(),
        params: Some(serde_json::json!({"input_columns": ["created_at"], "max_seconds": 86400})),
        transforms: Vec::new(),
    }));

    Plan {
        plan_version: "0.1".to_string(),
        seed: 123,
        schema_ref: SchemaRef {
            schema_version: "0.2".to_string(),
            schema_fingerprint: None,
            engine: "postgres".to_string(),
        },
        targets: vec![
            Target {
                schema: "public".to_string(),
                table: "users".to_string(),
                rows: 3,
                strategy: None,
            },
            Target {
                schema: "public".to_string(),
                table: "orders".to_string(),
                rows: 5,
                strategy: None,
            },
        ],
        rules,
        rules_unsupported: Vec::new(),
        options: Some(PlanOptions {
            allow_fk_disable: None,
            strict: Some(true),
        }),
    }
}

#[test]
fn golden_files_are_stable() {
    let schema = schema_fixture();
    let plan = plan_fixture();

    let out_dir = std::env::temp_dir().join(format!("datalchemy_golden_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let options = GenerateOptions {
        out_dir: out_dir.clone(),
        strict: true,
        max_attempts_row: 50,
        max_attempts_table: 3,
        auto_generate_parents: true,
    };
    let engine = GenerationEngine::new(options);
    let result = engine.run(&schema, &plan).expect("generation succeeds");

    let users_csv = result.run_dir.join("public.users.csv");
    let orders_csv = result.run_dir.join("public.orders.csv");

    let users_hash = hash_file(&users_csv).expect("hash users");
    let orders_hash = hash_file(&orders_csv).expect("hash orders");

    let expected_users = "c356748b70cbb9e719feb6da053340c245ad1230f8db4f081660d01f4a109911";
    let expected_orders = "7e445de7a7c92f6ca59441e7ec32017522d9d497479da034c7729b614b598bc8";

    assert_eq!(users_hash, expected_users, "users hash mismatch");
    assert_eq!(orders_hash, expected_orders, "orders hash mismatch");
}
