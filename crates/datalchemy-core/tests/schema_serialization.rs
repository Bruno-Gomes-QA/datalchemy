use datalchemy_core::{DatabaseSchema, Schema};

#[test]
fn serializes_schema_deterministically() {
    let schema = DatabaseSchema {
        schema_version: "0.2".to_string(),
        engine: "postgres".to_string(),
        database: Some("db".to_string()),
        schemas: vec![Schema {
            name: "public".to_string(),
            tables: Vec::new(),
        }],
        enums: Vec::new(),
        schema_fingerprint: None,
    };

    let json = serde_json::to_string_pretty(&schema).expect("serialize schema");
    let expected = r#"{
  "schema_version": "0.2",
  "engine": "postgres",
  "database": "db",
  "schemas": [
    {
      "name": "public",
      "tables": []
    }
  ],
  "enums": [],
  "schema_fingerprint": null
}"#;
    assert_eq!(json, expected);
}
