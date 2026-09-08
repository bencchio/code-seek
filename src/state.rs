use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn slot_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?.canonicalize().ok()?;
    let root = state_root()?;
    fs::create_dir_all(&root).ok()?;
    let name = allocate_name(&root, &cwd)?;
    let slot = root.join(name);
    fs::create_dir_all(&slot).ok()?;
    Some(slot)
}

fn state_root() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("code-seek"));
        }
    }
    let home = std::env::var_os("HOME")?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(".local/state/code-seek"))
}

fn index_path(root: &Path) -> PathBuf {
    root.join("index.json")
}

fn load_index(root: &Path) -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(index_path(root)) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_index(root: &Path, index: &HashMap<String, String>) {
    match serde_json::to_string(index) {
        Ok(json) => {
            if let Err(e) = fs::write(index_path(root), json) {
                crate::log::warn(&format!("failed to save state index: {e}"));
            }
        }
        Err(e) => crate::log::warn(&format!("failed to serialize state index: {e}")),
    }
}

fn slot_basename(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|s| s.to_str())
        .map(sanitize)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "project".to_owned())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn path_suffix(cwd: &Path) -> String {
    let hash = Sha256::digest(cwd.to_string_lossy().as_bytes());
    hash.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

fn allocate_name(root: &Path, cwd: &Path) -> Option<String> {
    let key = cwd.to_string_lossy().into_owned();
    let mut index = load_index(root);
    if let Some(existing) = index.get(&key) {
        return Some(existing.clone());
    }
    let base = slot_basename(cwd);
    let taken: std::collections::HashSet<&String> = index.values().collect();
    let name = if taken.contains(&base) {
        format!("{}-{}", base, path_suffix(cwd))
    } else {
        base
    };
    index.insert(key, name.clone());
    save_index(root, &index);
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_root(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("code_seek_state_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn first_project_uses_basename() {
        let root = unique_root("first");
        let state = root.join("code-seek");
        fs::create_dir_all(&state).unwrap();
        let cwd = root.join("myproj");
        fs::create_dir_all(&cwd).unwrap();
        let name = allocate_name(&state, &cwd.canonicalize().unwrap()).unwrap();
        assert_eq!(name, "myproj");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn colliding_basenames_get_a_suffix() {
        let root = unique_root("collide");
        let state = root.join("code-seek");
        fs::create_dir_all(&state).unwrap();
        let a = root.join("a").join("app");
        let b = root.join("b").join("app");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let na = allocate_name(&state, &a.canonicalize().unwrap()).unwrap();
        let nb = allocate_name(&state, &b.canonicalize().unwrap()).unwrap();
        assert_eq!(na, "app");
        assert_ne!(nb, na);
        assert!(nb.starts_with("app-"));
        let na2 = allocate_name(&state, &a.canonicalize().unwrap()).unwrap();
        assert_eq!(na2, na);
        let _ = fs::remove_dir_all(&root);
    }
}
