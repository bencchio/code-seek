use std::fs;

const DEFAULT_CONFIG: &str = r#"[scan]
follow_symlinks = false
max_file_size_mb = 10
ignore_dirs = [
    # Rust
    "target",
    # C / C++ / CMake
    "build", "cmake-build-debug", "cmake-build-release", "out",
    # JavaScript / TypeScript
    "node_modules", "dist", ".next", ".nuxt",
    # Python
    "__pycache__", ".venv", "venv", ".mypy_cache",
    # Java / Kotlin / Android
    ".gradle", "gradle", ".idea",
    # Swift / Objective-C
    "Pods", "DerivedData",
    # Dart / Flutter
    ".dart_tool",
    # Elixir
    "_build", "deps",
    # General
    "vendor", "coverage", ".cache", "tmp", "temp",
]
"#;

pub(crate) fn run(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dir = crate::state::slot_dir().ok_or("cannot resolve XDG state directory")?;
    let config_path = dir.join("config.toml");
    if config_path.exists() && !force {
        return Err(format!(
            "'{}' already exists. Use --force to overwrite.",
            config_path.display()
        )
        .into());
    }
    fs::create_dir_all(&dir)?;
    fs::write(&config_path, DEFAULT_CONFIG)?;
    println!("Initialized {} with config.toml", dir.display());
    Ok(())
}
