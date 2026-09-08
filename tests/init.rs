use std::fs;
use std::process::Command;

fn bin() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/target/debug/code-seek")
}

#[test]
fn init_creates_config() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_happy");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let out = Command::new(bin())
        .current_dir(&dir)
        .args(["init"])
        .output()
        .unwrap();

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(dir.join(".code-seek/config.toml").exists());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn init_fails_when_already_initialized() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_exists");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    Command::new(bin()).current_dir(&dir).args(["init"]).output().unwrap();

    let out = Command::new(bin())
        .current_dir(&dir)
        .args(["init"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("already exists"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn init_force_overwrites() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_force");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    Command::new(bin()).current_dir(&dir).args(["init"]).output().unwrap();

    let out = Command::new(bin())
        .current_dir(&dir)
        .args(["init", "--force"])
        .output()
        .unwrap();

    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(dir.join(".code-seek/config.toml").exists());

    fs::remove_dir_all(&dir).unwrap();
}
