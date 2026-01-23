use std::env;
use std::path::{Path, PathBuf};

use datalchemy_core::DatabaseSchema;
use datalchemy_plan::{ValidationReport, validate_plan};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut plan_path: Option<PathBuf> = None;
    let mut schema_path: Option<PathBuf> = None;
    let mut plan_schema_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--schema" => {
                schema_path = args.next().map(PathBuf::from);
            }
            "--plan-schema" => {
                plan_schema_path = args.next().map(PathBuf::from);
            }
            _ => {
                if plan_path.is_none() {
                    plan_path = Some(PathBuf::from(arg));
                } else {
                    return Err("unexpected argument".into());
                }
            }
        }
    }

    let plan_path = plan_path.ok_or("missing plan path")?;
    let schema_path = schema_path.ok_or("missing --schema path")?;
    let plan_schema_path =
        plan_schema_path.unwrap_or_else(|| PathBuf::from("schemas/plan.schema.json"));

    let plan_json = load_json(&plan_path)?;
    let plan_schema_json = load_json(&plan_schema_path)?;
    let schema_json = load_json(&schema_path)?;
    let schema: DatabaseSchema = serde_json::from_value(schema_json)?;

    let validated = match validate_plan(&plan_json, &plan_schema_json, &schema) {
        Ok(validated) => validated,
        Err(report) => {
            eprintln!("plan validation failed");
            print_report(&report);
            std::process::exit(1);
        }
    };

    if !validated.warnings.is_empty() {
        eprintln!("plan validated with warnings:");
        print_report(&ValidationReport {
            errors: Vec::new(),
            warnings: validated.warnings,
        });
    } else {
        println!("plan validated successfully");
    }

    Ok(())
}

fn load_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let json = serde_json::from_str(&contents)?;
    Ok(json)
}

fn print_report(report: &ValidationReport) {
    for issue in &report.errors {
        eprintln!("error {} {}: {}", issue.code, issue.path, issue.message);
        if let Some(hint) = &issue.hint {
            eprintln!("  hint: {hint}");
        }
    }
    for issue in &report.warnings {
        eprintln!("warning {} {}: {}", issue.code, issue.path, issue.message);
        if let Some(hint) = &issue.hint {
            eprintln!("  hint: {hint}");
        }
    }
}
