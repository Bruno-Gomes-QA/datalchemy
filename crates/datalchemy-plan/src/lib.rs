//! Plan contracts and validation for Plan 3.
//!
//! This crate defines the canonical `plan.json` structure, its JSON Schema,
//! and validation helpers (structural + schema-aware).

pub mod errors;
pub mod model;
pub mod schema;
pub mod validate;

pub use errors::{IssueSeverity, PlanError, ValidationIssue, ValidationReport};
pub use model::{
    ColumnGenerator, ColumnGeneratorRule, ConstraintKind, ConstraintMode, ConstraintPolicyRule,
    ForeignKeyMode, ForeignKeyStrategyRule, InsertOrder, Plan, PlanOptions, Rule, RuleReference,
    SchemaRef, Target, TargetStrategy, UnsupportedRule,
};
pub use schema::plan_json_schema;
pub use validate::{
    ValidatedPlan, validate_plan, validate_plan_against_schema, validate_plan_json,
};

/// Current plan contract version for `plan.json` artifacts.
pub const PLAN_VERSION: &str = "0.1";
