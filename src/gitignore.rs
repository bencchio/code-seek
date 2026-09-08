// Asks git which paths it would show, so scan can hide exactly what git hides.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The paths git is willing to show under a scan root.
pub(crate) struct Allowed {
    files: HashSet<PathBuf>,
    directories: HashSet<PathBuf>,
}

impl Allowed {
    pub(crate) fn allows_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }

    /// Directories are answered from the parents of the allowed files, so a
    /// directory holding nothing visible is pruned instead of descended into.
    pub(crate) fn allows_directory(&self, path: &Path) -> bool {
        self.directories.contains(path)
    }
}

/// `None` leaves scan unfiltered: the root is outside a repository, git is
/// unavailable, or it answered something this cannot read.
pub(crate) fn allowed(root: &Path) -> Option<Allowed> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let mut files = HashSet::new();
    let mut directories = HashSet::new();
    for entry in output.stdout.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        // Filtering on a path this cannot read would hide a real file, so the
        // whole answer is dropped and scan shows everything, with a warning.
        let Ok(relative) = std::str::from_utf8(entry) else {
            crate::log::warn("git reported a path that is not valid UTF-8; ignore rules are off");
            return None;
        };
        let path = root.join(relative);
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if !directories.insert(directory.to_path_buf()) {
                break;
            }
            parent = directory.parent();
        }
        files.insert(path);
    }

    Some(Allowed { files, directories })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;

    fn repository(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        root
    }

    #[test]
    fn ignored_file_is_not_allowed() {
        let root = repository("code_seek_gitignore_file");
        fs::write(root.join(".gitignore"), b"hidden.rs\n").unwrap();
        fs::write(root.join("shown.rs"), b"fn shown() {}").unwrap();
        fs::write(root.join("hidden.rs"), b"fn hidden() {}").unwrap();

        let allowed = allowed(&root).unwrap();
        assert!(allowed.allows_file(&root.join("shown.rs")));
        assert!(!allowed.allows_file(&root.join("hidden.rs")));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ignored_directory_is_not_allowed() {
        let root = repository("code_seek_gitignore_directory");
        fs::create_dir_all(root.join("vendored")).unwrap();
        fs::create_dir_all(root.join("kept")).unwrap();
        fs::write(root.join(".gitignore"), b"vendored/\n").unwrap();
        fs::write(root.join("vendored/dep.rs"), b"fn dep() {}").unwrap();
        fs::write(root.join("kept/main.rs"), b"fn main() {}").unwrap();

        let allowed = allowed(&root).unwrap();
        assert!(allowed.allows_directory(&root.join("kept")));
        assert!(!allowed.allows_directory(&root.join("vendored")));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn tracked_file_stays_allowed_despite_matching_a_pattern() {
        let root = repository("code_seek_gitignore_tracked");
        fs::write(root.join("kept.rs"), b"fn kept() {}").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["add", "kept.rs"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        fs::write(root.join(".gitignore"), b"kept.rs\n").unwrap();

        let allowed = allowed(&root).unwrap();
        assert!(allowed.allows_file(&root.join("kept.rs")));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn outside_a_repository_there_is_no_answer() {
        let root = std::env::temp_dir().join("code_seek_gitignore_plain");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.rs"), b"fn main() {}").unwrap();

        assert!(allowed(&root).is_none());

        fs::remove_dir_all(&root).unwrap();
    }
}
