use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::gitignore::Allowed;
use crate::lang;

const ALWAYS_IGNORED: &[&str] = &[".git", ".code-seek"];

/// `allowed` carries git's verdict; `None` walks everything, as before.
/// A root named explicitly is scanned whether or not git would show it.
pub(crate) fn walk(
    root: &Path,
    ignore_dirs: &[String],
    follow_symlinks: bool,
    allowed: Option<&Allowed>,
) -> Vec<PathBuf> {
    if !follow_symlinks && root.is_symlink() {
        return Vec::new();
    }
    if root.is_file() {
        if lang::detect(root).is_some() {
            return vec![root.to_path_buf()];
        }
        return Vec::new();
    }
    let mut files = Vec::with_capacity(256);
    let mut visited = HashSet::new();
    walk_dir(
        root,
        &mut files,
        ignore_dirs,
        follow_symlinks,
        &mut visited,
        allowed,
    );
    files.sort();
    files
}

fn walk_dir(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    ignore_dirs: &[String],
    follow_symlinks: bool,
    visited: &mut HashSet<PathBuf>,
    allowed: Option<&Allowed>,
) {
    let real = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(real) {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            crate::log::warn(&format!("skipping '{}': {}", dir.display(), e));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !follow_symlinks && path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if ALWAYS_IGNORED.contains(&name) || ignore_dirs.iter().any(|d| d == name) {
                continue;
            }
            if let Some(allowed) = allowed
                && !allowed.allows_directory(&path)
            {
                continue;
            }
            walk_dir(&path, files, ignore_dirs, follow_symlinks, visited, allowed);
        } else if lang::detect(&path).is_some() {
            if let Some(allowed) = allowed
                && !allowed.allows_file(&path)
            {
                continue;
            }
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn single_file_returns_itself() {
        let tmp = std::env::temp_dir().join("code_seek_walker_single.c");
        fs::write(&tmp, b"int x;").unwrap();
        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], tmp);
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn unsupported_single_file_returns_empty() {
        let tmp = std::env::temp_dir().join("code_seek_walker_single.toml");
        fs::write(&tmp, b"[section]").unwrap();
        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 0);
        fs::remove_file(&tmp).unwrap();
    }

    #[test]
    fn ignores_git_and_code_seek() {
        let tmp = std::env::temp_dir().join("code_seek_walker_test");
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join(".git")).unwrap();
        fs::create_dir_all(tmp.join(".code-seek")).unwrap();
        fs::write(tmp.join("foo.c"), b"int main() {}").unwrap();
        fs::write(tmp.join(".git/HEAD"), b"ref: refs/heads/main").unwrap();
        fs::write(tmp.join(".code-seek/config.toml"), b"version = '0.1'").unwrap();

        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap() == "foo.c");

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn ignores_config_dirs() {
        let tmp = std::env::temp_dir().join("code_seek_walker_ignore");
        fs::create_dir_all(tmp.join("target/debug")).unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join("src/main.rs"), b"fn main() {}").unwrap();
        fs::write(tmp.join("target/debug/build.rlib"), b"binary").unwrap();

        let files = walk(&tmp, &["target".to_owned()], false, None);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/main.rs"));

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn skips_unsupported_extensions() {
        let tmp = std::env::temp_dir().join("code_seek_walker_exts");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("main.rs"), b"fn main() {}").unwrap();
        fs::write(tmp.join("Makefile"), b"all:").unwrap();
        fs::write(tmp.join("README.md"), b"# Readme").unwrap();

        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap() == "main.rs");

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn empty_directory_returns_empty() {
        let tmp = std::env::temp_dir().join("code_seek_walker_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 0);
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn ignore_dirs_case_sensitive() {
        let tmp = std::env::temp_dir().join("code_seek_walker_case");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("Target")).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::write(tmp.join("Target/main.rs"), b"fn main() {}").unwrap();
        fs::write(tmp.join("target/build.rs"), b"fn main() {}").unwrap();
        let files = walk(&tmp, &["target".to_owned()], false, None);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().contains("Target"));
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn git_verdict_prunes_directories_and_files() {
        let tmp = std::env::temp_dir().join("code_seek_walker_gitignore");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("vendored")).unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(&tmp)
            .arg("init")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        fs::write(tmp.join(".gitignore"), b"vendored/\nhidden.rs\n").unwrap();
        fs::write(tmp.join("shown.rs"), b"fn shown() {}").unwrap();
        fs::write(tmp.join("hidden.rs"), b"fn hidden() {}").unwrap();
        fs::write(tmp.join("vendored/dep.rs"), b"fn dep() {}").unwrap();

        let allowed = crate::gitignore::allowed(&tmp).unwrap();
        let files = walk(&tmp, &[], false, Some(&allowed));
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("shown.rs"));

        let unfiltered = walk(&tmp, &[], false, None);
        assert_eq!(unfiltered.len(), 3);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_file_skipped_when_not_following() {
        use std::os::unix::fs::symlink;
        let tmp = std::env::temp_dir().join("code_seek_walker_symlink_no_follow");
        fs::create_dir_all(&tmp).unwrap();
        let real = tmp.join("real.rs");
        let link = tmp.join("link.rs");
        fs::write(&real, b"fn main() {}").unwrap();
        symlink(&real, &link).unwrap();

        let files = walk(&tmp, &[], false, None);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "real.rs");

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_file_included_when_following() {
        use std::os::unix::fs::symlink;
        let tmp = std::env::temp_dir().join("code_seek_walker_symlink_follow");
        fs::create_dir_all(&tmp).unwrap();
        let real = tmp.join("real.rs");
        let link = tmp.join("link.rs");
        fs::write(&real, b"fn main() {}").unwrap();
        symlink(&real, &link).unwrap();

        let files = walk(&tmp, &[], true, None);
        assert_eq!(files.len(), 2);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlink_single_file_skipped_when_not_following() {
        use std::os::unix::fs::symlink;
        let tmp = std::env::temp_dir().join("code_seek_walker_symlink_single");
        fs::create_dir_all(&tmp).unwrap();
        let real = tmp.join("real.rs");
        let link = tmp.join("link.rs");
        fs::write(&real, b"fn main() {}").unwrap();
        symlink(&real, &link).unwrap();

        let files = walk(&link, &[], false, None);
        assert_eq!(files.len(), 0);

        fs::remove_dir_all(&tmp).unwrap();
    }
}
