//! Rule-based data generation engine for Datalchemy (Plan 4).
//!
//! This crate consumes `schema.json` + `plan.json` to produce deterministic
//! datasets (CSV) with constraint-aware generation.

// Allow large error variants temporarily - see issue_task_20260128_refactor_errors
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

pub mod assets;
pub mod checks;
pub mod engine;
pub mod errors;
pub mod faker_rs;
pub mod foreign;
pub mod generators;
pub mod model;
pub mod output;
pub mod params;
pub mod planner;

pub use engine::{GenerationEngine, GenerationResult};
pub use errors::GenerationError;
pub use model::{GenerateOptions, GenerationReport, TableReport};
