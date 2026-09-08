use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) scan: ScanConfig,
}

fn default_max_file_size_mb() -> f64 { 10.0 }

#[derive(Deserialize)]
pub(crate) struct ScanConfig {
    #[serde(default)]
    pub(crate) ignore_dirs: Vec<String>,
    #[serde(default)]
    pub(crate) follow_symlinks: bool,
    #[serde(default = "default_max_file_size_mb")]
    pub(crate) max_file_size_mb: f64,
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

pub(crate) fn load() -> Config {
    let path = Path::new(".code-seek/config.toml");
    let Ok(content) = std::fs::read_to_string(path) else {
        return Config::default();
    };
    toml::from_str(&content).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let cfg = Config::default();
        assert!(!cfg.scan.follow_symlinks);
        assert_eq!(cfg.scan.max_file_size_mb, 10.0);
        assert!(cfg.scan.ignore_dirs.is_empty());
    }

    #[test]
    fn parses_valid_toml() {
        let cfg: Config =
            toml::from_str("[scan]\nfollow_symlinks = true\nmax_file_size_mb = 5.0").unwrap();
        assert!(cfg.scan.follow_symlinks);
        assert_eq!(cfg.scan.max_file_size_mb, 5.0);
    }

    #[test]
    fn invalid_toml_produces_error() {
        let result: Result<Config, _> = toml::from_str("not valid toml [[[");
        assert!(result.is_err());
    }
}
