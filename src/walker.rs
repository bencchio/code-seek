use std::path::{Path, PathBuf};

const IGNORED_DIRS: &[&str] = &[".git", ".code-seek"];

pub fn walk(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(root, &mut files);
    files.sort();
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if IGNORED_DIRS.contains(&name) {
                continue;
            }
            walk_dir(&path, files);
        } else {
            files.push(path);
        }
    }
}
