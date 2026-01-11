use datalchemy_core::DatabaseSchema;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DatabaseSchema);
    let json = serde_json::to_string_pretty(&schema).expect("serialize json schema");
    println!("{json}");
}
