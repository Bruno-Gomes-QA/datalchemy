use std::env;
use std::path::PathBuf;

use datalchemy_core::DatabaseSchema;
use datalchemy_generate::{GenerateOptions, GenerationEngine};
use datalchemy_plan::Plan;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let mut args = env::args().skip(1);
    let mut plan_path: Option<PathBuf> = None;
    let mut schema_path: Option<PathBuf> = None;
    let mut out_dir: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--plan" => plan_path = args.next().map(PathBuf::from),
            "--schema" => schema_path = args.next().map(PathBuf::from),
            "--out" => out_dir = args.next().map(PathBuf::from),
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
    let plan_json = std::fs::read_to_string(&plan_path)?;
    let schema_json = std::fs::read_to_string(&schema_path)?;

    let plan: Plan = serde_json::from_str(&plan_json)?;
    let schema: DatabaseSchema = serde_json::from_str(&schema_json)?;

    let mut options = GenerateOptions::default();
    if let Some(out_dir) = out_dir {
        options.out_dir = out_dir;
    }

    let engine = GenerationEngine::new(options);
    let result = engine.run(&schema, &plan)?;

    println!("run_dir={}", result.run_dir.display());
    Ok(())
}
