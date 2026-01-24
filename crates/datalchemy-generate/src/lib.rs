//! Rule-based data generation engine for Datalchemy (Plan 4).
//!
//! This crate consumes `schema.json` + `plan.json` to produce deterministic
//! datasets (CSV) with constraint-aware generation.

pub mod assets;
pub mod checks;
pub mod engine;
pub mod errors;
pub mod foreign;
pub mod generators;
pub mod model;
pub mod output;
pub mod planner;

pub use engine::{GenerationEngine, GenerationResult};
pub use errors::GenerationError;
pub use model::{GenerateOptions, GenerationReport, TableReport};
