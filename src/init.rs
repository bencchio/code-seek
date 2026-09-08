use std::fs;
use std::path::Path;

const DEFAULT_CONFIG: &str = r#"version = "0.1"

[scan]
follow_symlinks = false
max_file_size_mb = 10
save_history_by_default = false
"#;

pub fn run(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(".code-seek");
    if dir.exists() && !force {
        return Err("'.code-seek/' already exists. Use --force to overwrite.".into());
    }
    fs::create_dir_all(dir)?;
    fs::write(dir.join("config.toml"), DEFAULT_CONFIG)?;
    println!("Initialized .code-seek/ with config.toml");
    Ok(())
}
