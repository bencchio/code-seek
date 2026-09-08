use std::fs;
use std::process::Command;

fn bin() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/target/debug/code-seek")
}

#[test]
fn scan_c_file() {
    let dir = std::env::temp_dir().join("code_seek_inttest_c");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("test.c"),
        "int add(int a, int b) { return a + b; }",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("[C]"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_cpp_file() {
    let dir = std::env::temp_dir().join("code_seek_inttest_cpp");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("test.cpp"), "class Foo { void bar() {} };").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("[C++]"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_single_c_file() {
    let file = std::env::temp_dir().join("code_seek_inttest_single.c");
    fs::write(&file, "int add(int a, int b) { return a + b; }").unwrap();

    let out = Command::new(bin())
        .args(["scan", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[C]"));
    assert!(stdout.contains("1 entity"));

    fs::remove_file(&file).unwrap();
}

#[test]
fn scan_rust_file() {
    let dir = std::env::temp_dir().join("code_seek_inttest_rust");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("test.rs"),
        "fn hello() -> u32 { 42 }\nstruct Foo { x: i32 }\nimpl Foo { fn get(&self) -> i32 { self.x } }",
    ).unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[Rust]"));
    assert!(stdout.contains("4 entities"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_qml_file() {
    let dir = std::env::temp_dir().join("code_seek_inttest_qml");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("test.qml"),
        "import QtQuick 2.0\nRectangle {\n    id: root\n    function reset() { }\n    function update(x) { }\n}",
    ).unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[QML]"));
    assert!(stdout.contains("3 entities"));

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_nonexistent_path() {
    let out = Command::new(bin())
        .args(["scan", "/nonexistent_code-seek_path"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("does not exist"));
}
