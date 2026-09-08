use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub scan: ScanConfig,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    pub ignore_dirs: Vec<String>,
    pub follow_symlinks: bool,
    pub max_file_size_mb: f64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            ignore_dirs: Vec::new(),
            follow_symlinks: false,
            max_file_size_mb: 10.0,
        }
    }
}

pub fn load() -> Config {
    let path = Path::new(".code-seek/config.toml");
    let Ok(content) = std::fs::read_to_string(path) else {
        return Config::default();
    };
    toml::from_str(&content).unwrap_or_default()
}
