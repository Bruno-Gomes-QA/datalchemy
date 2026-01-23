use schemars::schema::RootSchema;
use schemars::schema_for;

use crate::model::Plan;

/// Emit the JSON Schema for `plan.json`.
pub fn plan_json_schema() -> RootSchema {
    schema_for!(Plan)
}
