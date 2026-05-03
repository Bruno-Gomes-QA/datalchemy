use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use crate::errors::GenerationError;

#[derive(Debug, Clone)]
struct AssetLines {
    values: Vec<String>,
    missing: bool,
}

#[derive(Debug, Clone)]
enum AssetEntry {
    Lines(Vec<String>),
    Missing,
}

#[derive(Debug)]
pub struct AssetsLoader {
    root: PathBuf,
    cache: RwLock<BTreeMap<String, AssetEntry>>,
}

impl AssetsLoader {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn load_lines(&self, relative: &str) -> Result<Vec<String>, GenerationError> {
        let entry = self.load_entry(relative, Self::read_lines)?;
        Ok(entry.values)
    }

    pub fn load_json_strings(&self, relative: &str) -> Result<Vec<String>, GenerationError> {
        let entry = self.load_entry(relative, Self::read_json_strings)?;
        Ok(entry.values)
    }

    pub fn asset_missing(&self, relative: &str) -> bool {
        let cache = self.cache.read().ok();
        if let Some(cache) = cache
            && let Some(AssetEntry::Missing) = cache.get(relative)
        {
            return true;
        }
        false
    }

    fn load_entry<F>(&self, relative: &str, loader: F) -> Result<AssetLines, GenerationError>
    where
        F: Fn(&Path) -> Result<AssetLines, GenerationError>,
    {
        if let Some(entry) = self.cached(relative) {
            return Ok(entry);
        }

        let path = self.root.join(relative);
        let entry = loader(&path)?;

        let mut cache = self
            .cache
            .write()
            .map_err(|_| GenerationError::Asset("asset cache poisoned".to_string()))?;
        cache.insert(
            relative.to_string(),
            if entry.missing {
                AssetEntry::Missing
            } else {
                AssetEntry::Lines(entry.values.clone())
            },
        );

        Ok(entry)
    }

    fn cached(&self, relative: &str) -> Option<AssetLines> {
        let cache = self.cache.read().ok()?;
        match cache.get(relative)? {
            AssetEntry::Lines(values) => Some(AssetLines {
                values: values.clone(),
                missing: false,
            }),
            AssetEntry::Missing => Some(AssetLines {
                values: Vec::new(),
                missing: true,
            }),
        }
    }

    fn read_lines(path: &Path) -> Result<AssetLines, GenerationError> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(AssetLines {
                        values: Vec::new(),
                        missing: true,
                    });
                }
                return Err(GenerationError::Asset(format!(
                    "failed to read asset {}: {}",
                    path.display(),
                    err
                )));
            }
        };

        let values = contents
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(AssetLines {
            values,
            missing: false,
        })
    }

    fn read_json_strings(path: &Path) -> Result<AssetLines, GenerationError> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Ok(AssetLines {
                        values: Vec::new(),
                        missing: true,
                    });
                }
                return Err(GenerationError::Asset(format!(
                    "failed to read asset {}: {}",
                    path.display(),
                    err
                )));
            }
        };

        let values: Vec<String> = serde_json::from_str(&contents).map_err(|err| {
            GenerationError::Asset(format!("invalid json asset {}: {}", path.display(), err))
        })?;

        Ok(AssetLines {
            values,
            missing: false,
        })
    }
}

pub fn assets_loader() -> &'static AssetsLoader {
    static LOADER: OnceLock<AssetsLoader> = OnceLock::new();
    LOADER.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        AssetsLoader::new(root)
    })
}
