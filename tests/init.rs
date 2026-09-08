use std::fs;
use std::process::Command;

fn code_seek() -> std::path::PathBuf {
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/target");
    let debug = target.join("debug/code-seek");
    if debug.exists() {
        return debug;
    }
    target.join("release/code-seek")
}

fn command() -> Command {
    let state = std::env::temp_dir().join("code_seek_test_xdg_state");
    let _ = fs::create_dir_all(&state);
    let mut c = Command::new(code_seek());
    c.env("XDG_STATE_HOME", &state);
    c
}

fn slot_config(project: &std::path::Path) -> std::path::PathBuf {
    let state = std::env::temp_dir().join("code_seek_test_xdg_state");
    let name = project.file_name().unwrap();
    state.join("code-seek").join(name).join("config.toml")
}

#[test]
fn init_creates_config() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_happy");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let _ = fs::remove_file(slot_config(&dir));

    let out = command().current_dir(&dir).args(["init"]).output().unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(slot_config(&dir).exists());
    assert!(!dir.join(".code-seek").exists());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn init_fails_when_already_initialized() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_exists");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    command().current_dir(&dir).args(["init"]).output().unwrap();

    let out = command().current_dir(&dir).args(["init"]).output().unwrap();

    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("already exists"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn init_force_overwrites() {
    let dir = std::env::temp_dir().join("code_seek_inttest_init_force");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    command().current_dir(&dir).args(["init"]).output().unwrap();

    let out = command()
        .current_dir(&dir)
        .args(["init", "--force"])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(slot_config(&dir).exists());
    assert!(!dir.join(".code-seek").exists());

    fs::remove_dir_all(&dir).unwrap();
}
