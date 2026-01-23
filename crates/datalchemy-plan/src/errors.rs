use thiserror::Error;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

/// Structured validation issue with location and hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub path: String,
    pub message: String,
    pub hint: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue.
    pub fn new(
        severity: IssueSeverity,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            path: path.into(),
            message: message.into(),
            hint,
        }
    }
}

/// Aggregated validation report with errors and warnings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Returns true when there are no errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Add an error issue.
    pub fn push_error(&mut self, issue: ValidationIssue) {
        self.errors.push(issue);
    }

    /// Add a warning issue.
    pub fn push_warning(&mut self, issue: ValidationIssue) {
        self.warnings.push(issue);
    }

    /// Merge another report into this one.
    pub fn merge(&mut self, other: ValidationReport) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Plan validation errors that are not structural issues.
#[derive(Debug, Error)]
pub enum PlanError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema error: {0}")]
    Schema(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for plan validation operations.
pub type Result<T> = std::result::Result<T, PlanError>;
