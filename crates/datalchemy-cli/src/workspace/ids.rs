use chrono::Utc;

pub fn new_artifact_id(kind: &str) -> String {
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let short = short_id();
    format!("{date}__{kind}_{short}")
}

fn short_id() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    match id.split('-').next() {
        Some(part) if !part.is_empty() => part.to_string(),
        _ => id,
    }
}
