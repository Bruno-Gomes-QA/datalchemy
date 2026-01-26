use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WriteIntent {
    pub reason: String,
    pub paths: Vec<PathBuf>,
}

impl WriteIntent {
    pub fn new(reason: impl Into<String>, paths: Vec<PathBuf>) -> Self {
        Self {
            reason: reason.into(),
            paths,
        }
    }
}
