use std::env;
use std::path::PathBuf;

use datalchemy_core::DatabaseSchema;
use datalchemy_eval::{EvaluateOptions, EvaluationEngine};
use datalchemy_plan::Plan;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut plan_path: Option<PathBuf> = None;
    let mut schema_path: Option<PathBuf> = None;
    let mut run_dir: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--plan" => plan_path = args.next().map(PathBuf::from),
            "--schema" => schema_path = args.next().map(PathBuf::from),
            "--run" => run_dir = args.next().map(PathBuf::from),
            _ => {
                if plan_path.is_none() {
                    plan_path = Some(PathBuf::from(arg));
                } else {
                    return Err("unexpected argument".into());
                }
            }
        }
    }

    let plan_path = plan_path.ok_or("missing --plan path")?;
    let schema_path = schema_path.ok_or("missing --schema path")?;
    let run_dir = run_dir.ok_or("missing --run directory")?;

    let plan_json = std::fs::read_to_string(&plan_path)?;
    let schema_json = std::fs::read_to_string(&schema_path)?;

    let plan: Plan = serde_json::from_str(&plan_json)?;
    let schema: DatabaseSchema = serde_json::from_str(&schema_json)?;

    let options = EvaluateOptions::default();
    let engine = EvaluationEngine::new(options);
    let result = engine.run(&schema, &plan, &run_dir)?;

    println!("metrics_path={}", result.metrics_path.display());
    println!("report_path={}", result.report_path.display());
    if let Some(path) = result.violations_path {
        println!("violations_path={}", path.display());
    }
    Ok(())
}
