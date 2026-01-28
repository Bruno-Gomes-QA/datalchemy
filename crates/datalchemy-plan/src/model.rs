use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reference to the schema used when the plan was authored.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchemaRef {
    /// Schema contract version expected by the plan.
    pub schema_version: String,
    /// Optional schema fingerprint to prevent drift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_fingerprint: Option<String>,
    /// Engine identifier (ex.: postgres).
    pub engine: String,
}

/// A target table and expected row count.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    /// Schema name (namespace) of the table.
    pub schema: String,
    /// Table name within the schema.
    pub table: String,
    /// Number of rows to generate.
    pub rows: u64,
    /// Optional strategy hints for generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<TargetStrategy>,
}

/// Optional strategy hints for a target.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetStrategy {
    /// Insert order strategy (future support).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insert_order: Option<InsertOrder>,
    /// Suggested batch size for inserts (future support).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
}

/// Supported insert ordering strategies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InsertOrder {
    /// Respect foreign-key topological ordering.
    FkToposort,
}

/// Rule union for plan-level instructions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rule {
    /// Provide a generator for a specific column.
    ColumnGenerator(ColumnGeneratorRule),
    /// Declare constraint handling policy for a table.
    ConstraintPolicy(ConstraintPolicyRule),
    /// Configure how foreign keys are handled per table.
    ForeignKeyStrategy(ForeignKeyStrategyRule),
}

/// Column generator rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnGeneratorRule {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub generator: GeneratorRef,
    /// Legacy generator parameters (deprecated; prefer generator.params).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Optional transforms applied after generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<TransformRule>,
}

/// Generator reference; accepts legacy string id or full spec.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum GeneratorRef {
    Id(String),
    Spec(GeneratorSpec),
}

/// Generator spec with optional locale and params.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeneratorSpec {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl GeneratorRef {
    pub fn id(&self) -> &str {
        match self {
            GeneratorRef::Id(value) => value.as_str(),
            GeneratorRef::Spec(spec) => spec.id.as_str(),
        }
    }

    pub fn locale(&self) -> Option<&str> {
        match self {
            GeneratorRef::Id(_) => None,
            GeneratorRef::Spec(spec) => spec.locale.as_deref(),
        }
    }

    pub fn params(&self) -> Option<&serde_json::Value> {
        match self {
            GeneratorRef::Id(_) => None,
            GeneratorRef::Spec(spec) => spec.params.as_ref(),
        }
    }
}

impl ColumnGeneratorRule {
    pub fn generator_id(&self) -> &str {
        self.generator.id()
    }

    pub fn generator_locale(&self) -> Option<&str> {
        self.generator.locale()
    }

    pub fn generator_params(&self) -> Option<&serde_json::Value> {
        self.generator.params().or(self.params.as_ref())
    }

    pub fn normalized_generator(&self) -> GeneratorSpec {
        GeneratorSpec {
            id: self.generator.id().to_string(),
            locale: self.generator.locale().map(|value| value.to_string()),
            params: self
                .generator
                .params()
                .cloned()
                .or_else(|| self.params.clone()),
        }
    }
}

/// Transform rule applied to generated values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TransformRule {
    pub transform: String,
    /// Transform parameters (shape depends on the transform).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Constraint policy rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConstraintPolicyRule {
    pub schema: String,
    pub table: String,
    pub constraint: ConstraintKind,
    pub mode: ConstraintMode,
}

/// Constraint categories that can be controlled by policy.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKind {
    Check,
    Unique,
    NotNull,
    PrimaryKey,
    ForeignKey,
}

/// Policy for constraint handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintMode {
    Enforce,
    Warn,
    Ignore,
}

/// Foreign key strategy rule.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForeignKeyStrategyRule {
    pub schema: String,
    pub table: String,
    pub mode: ForeignKeyMode,
}

/// Foreign key strategy modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForeignKeyMode {
    Respect,
    Disable,
}

/// Unsupported rule placeholder for future features.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnsupportedRule {
    /// Short description of the intent.
    pub description: String,
    /// Reason why it is unsupported today.
    pub reason: String,
    /// Optional reference to a schema/table/column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<RuleReference>,
}

/// Reference to schema/table/column for rules.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuleReference {
    pub schema: String,
    pub table: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
}

/// Optional plan-level options (reserved for future use).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanOptions {
    /// Allow disabling foreign-key enforcement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fk_disable: Option<bool>,
    /// Enable strict generation mode (fallbacks become errors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Optional plan-level globals shared by all rules.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanGlobal {
    /// Default locale for generators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Canonical plan definition for generation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    /// Contract version for the plan format.
    pub plan_version: String,
    /// Seed for reproducibility.
    pub seed: u64,
    /// Reference to the schema used when authoring the plan.
    pub schema_ref: SchemaRef,
    /// Optional plan-level globals (locale, etc).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global: Option<PlanGlobal>,
    /// Targets to generate.
    pub targets: Vec<Target>,
    /// Rules that apply to tables/columns.
    pub rules: Vec<Rule>,
    /// Unsupported rules recorded for future evolution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules_unsupported: Vec<UnsupportedRule>,
    /// Optional plan-level options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<PlanOptions>,
}
