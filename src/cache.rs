use crate::{
    lang,
    model::{Dependency, Entity, FileResult, SyntaxError},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::{Path, PathBuf}};

#[derive(Serialize, Deserialize)]
struct CachedEntry {
    sha256: String,
    language: String,
    loc: usize,
    entities: Vec<Entity>,
    imports: Vec<String>,
    dependencies: Vec<Dependency>,
    errors: Vec<SyntaxError>,
}

pub(crate) struct Cache {
    entries: HashMap<String, CachedEntry>,
}

pub(crate) fn sha256(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn load(path: &Path) -> Cache {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Cache { entries: HashMap::new() },
    };
    let entries: HashMap<String, CachedEntry> = match serde_json::from_str(&content) {
        Ok(e) => e,
        Err(_) => {
            crate::log::warn("cache file is corrupted, starting fresh");
            HashMap::new()
        }
    };
    Cache { entries }
}

impl Cache {
    pub(crate) fn get(&self, key: &str, sha256: &str) -> Option<FileResult> {
        let entry = self.entries.get(key)?;
        if entry.sha256 != sha256 {
            return None;
        }
        let language = lang::LANGUAGES
            .iter()
            .find(|l| l.canonical == entry.language.as_str())?
            .canonical;
        Some(FileResult {
            path: PathBuf::from(key),
            language,
            loc: entry.loc,
            entities: entry.entities.clone(),
            imports: entry.imports.clone(),
            dependencies: entry.dependencies.clone(),
            errors: entry.errors.clone(),
        })
    }

    pub(crate) fn insert(&mut self, key: String, result: &FileResult, sha256: String) {
        self.entries.insert(
            key,
            CachedEntry {
                sha256,
                language: result.language.to_owned(),
                loc: result.loc,
                entities: result.entities.clone(),
                imports: result.imports.clone(),
                dependencies: result.dependencies.clone(),
                errors: result.errors.clone(),
            },
        );
    }

    pub(crate) fn save(&self, path: &Path) {
        if path.parent().is_some_and(|p| !p.as_os_str().is_empty() && !p.exists()) {
            return;
        }
        match serde_json::to_string(&self.entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    crate::log::warn(&format!("failed to save cache: {e}"));
                }
            }
            Err(e) => crate::log::warn(&format!("failed to serialize cache: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file_result() -> FileResult {
        FileResult {
            path: PathBuf::from("src/lib.rs"),
            language: "Rust",
            loc: 5,
            entities: vec![],
            imports: vec![],
            dependencies: vec![],
            errors: vec![],
        }
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = Cache { entries: HashMap::new() };
        assert!(cache.get("src/lib.rs", "abc123").is_none());
    }

    #[test]
    fn cache_hash_mismatch_returns_none() {
        let mut cache = Cache { entries: HashMap::new() };
        cache.insert("src/lib.rs".to_owned(), &make_file_result(), "correct_hash".to_owned());
        assert!(cache.get("src/lib.rs", "wrong_hash").is_none());
    }

    #[test]
    fn cache_hit_returns_file_result() {
        let mut cache = Cache { entries: HashMap::new() };
        cache.insert("src/lib.rs".to_owned(), &make_file_result(), "hash1".to_owned());
        let got = cache.get("src/lib.rs", "hash1").unwrap();
        assert_eq!(got.language, "Rust");
        assert_eq!(got.loc, 5);
        assert_eq!(got.path, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("code-seek_cache_unit");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cache_path = dir.join("cache.json");

        let mut cache = Cache { entries: HashMap::new() };
        cache.insert("src/lib.rs".to_owned(), &make_file_result(), "myhash".to_owned());
        cache.save(&cache_path);

        let loaded = load(&cache_path);
        let got = loaded.get("src/lib.rs", "myhash").unwrap();
        assert_eq!(got.loc, 5);
        assert_eq!(got.language, "Rust");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cache_load_corrupt_json() {
        let path = std::env::temp_dir().join("code-seek_cache_corrupt_unit.json");
        std::fs::write(&path, b"{{{bad json").unwrap();
        let cache = load(&path);
        assert!(cache.get("anything", "hash").is_none());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn cache_load_missing_file() {
        let cache = load(std::path::Path::new("/nonexistent/code-seek_cache_unit.json"));
        assert!(cache.get("anything", "hash").is_none());
    }

    #[test]
    fn cache_insert_empty_result() {
        let mut cache = Cache { entries: HashMap::new() };
        let r = FileResult {
            path: PathBuf::from("empty.rs"),
            language: "Rust",
            loc: 0,
            entities: vec![],
            imports: vec![],
            dependencies: vec![],
            errors: vec![],
        };
        cache.insert("empty.rs".to_owned(), &r, "emptyhash".to_owned());
        let got = cache.get("empty.rs", "emptyhash").unwrap();
        assert_eq!(got.entities.len(), 0);
        assert_eq!(got.imports.len(), 0);
        assert_eq!(got.dependencies.len(), 0);
        assert_eq!(got.errors.len(), 0);
        assert_eq!(got.loc, 0);
    }
}
