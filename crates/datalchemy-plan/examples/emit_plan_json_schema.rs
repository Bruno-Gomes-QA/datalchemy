use datalchemy_plan::plan_json_schema;

fn main() {
    let schema = plan_json_schema();
    let json = serde_json::to_string_pretty(&schema).expect("serialize plan json schema");
    println!("{json}");
}
