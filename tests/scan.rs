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

#[test]
fn scan_json_format_produces_valid_json() {
    let dir = std::env::temp_dir().join("code_seek_inttest_json");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }\nstruct Point { x: f32, y: f32 }",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("output is not valid JSON");

    assert_eq!(json["total_files"], 1);
    assert_eq!(json["total_entities"], 2);
    assert!(json["files"].is_array());
    assert_eq!(json["files"][0]["language"], "Rust");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_json_entity_paths_are_correct() {
    let dir = std::env::temp_dir().join("code_seek_inttest_json_paths");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("foo.rs"),
        "impl Foo {\n    fn bar(&self) {}\n}\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // impl Foo → entity_path contains "impl Foo"
    let impl_entity = &json["files"][0]["entities"][0];
    assert_eq!(impl_entity["entity_type"], "Impl");
    let impl_path = impl_entity["entity_path"].as_str().unwrap();
    assert!(impl_path.ends_with("> impl Foo"), "got: {impl_path}");

    // method bar → entity_path is "... > impl Foo > bar"
    let method = &impl_entity["children"][0];
    assert_eq!(method["entity_type"], "Method");
    let method_path = method["entity_path"].as_str().unwrap();
    assert!(method_path.ends_with("> impl Foo > bar"), "got: {method_path}");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filter_match_includes_matching_excludes_others() {
    let dir = std::env::temp_dir().join("code_seek_inttest_match");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("lib.rs"), "fn add(a: i32, b: i32) -> i32 { a + b }\nfn sub(a: i32, b: i32) -> i32 { a - b }").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--match", "add"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("add"), "expected 'add' in output");
    assert!(!stdout.contains("sub"), "expected 'sub' to be filtered out");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filter_match_preserves_parent_impl() {
    let dir = std::env::temp_dir().join("code_seek_inttest_match_tree");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("foo.rs"),
        "impl Foo {\n    fn bar(&self) {}\n    fn baz(&self) {}\n}\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--match", "bar"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Foo"), "parent impl Foo must be preserved");
    assert!(stdout.contains("bar"), "matching child must appear");
    assert!(!stdout.contains("baz"), "non-matching child must be excluded");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filter_max_depth_truncates_children() {
    let dir = std::env::temp_dir().join("code_seek_inttest_depth");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("foo.rs"),
        "impl Foo {\n    fn bar(&self) {}\n}\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--max-depth", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Foo"), "root entity must appear at depth 1");
    assert!(!stdout.contains("bar"), "child at depth 2 must be pruned");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filter_empty_match_shows_file_header() {
    let dir = std::env::temp_dir().join("code_seek_inttest_match_empty");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("lib.rs"), "fn add() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--match", "zzznomatch"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("[Rust]"), "file header must still appear with 0 matches");
    assert!(stdout.contains("0 entities"), "entity count must be 0");
    assert!(!stdout.contains("add"), "non-matching entity must not appear");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filter_combined_match_and_depth() {
    let dir = std::env::temp_dir().join("code_seek_inttest_combined");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("main.rs"),
        "fn main() {}\nfn helper() {}\nstruct Config {}\n",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--match", "main", "--max-depth", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("main"), "matching entity must appear");
    assert!(!stdout.contains("helper"), "non-matching entity must be absent");
    assert!(!stdout.contains("Config"), "non-matching entity must be absent");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_uses_cache_on_second_run() {
    let dir = std::env::temp_dir().join("code_seek_inttest_cache");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(dir.join(".code-seek")).unwrap();
    fs::write(
        dir.join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }\nstruct Point { x: f32 }",
    )
    .unwrap();

    // First run — parses the file, writes cache
    let out1 = Command::new(bin())
        .args(["scan", "."])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out1.status.success(), "first scan failed: {}", String::from_utf8_lossy(&out1.stderr));
    let stdout1 = String::from_utf8_lossy(&out1.stdout).to_string();

    let cache_path = dir.join(".code-seek/cache.json");
    assert!(cache_path.exists(), "cache.json must be created after first scan");

    let cache_contents = fs::read_to_string(&cache_path).unwrap();
    let cache_json: serde_json::Value =
        serde_json::from_str(&cache_contents).expect("cache is valid JSON");
    assert!(
        cache_json.as_object().map(|o| !o.is_empty()).unwrap_or(false),
        "cache must contain entries",
    );

    // Second run — reads from cache, no re-parse
    let out2 = Command::new(bin())
        .args(["scan", "."])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out2.status.success(), "second scan failed: {}", String::from_utf8_lossy(&out2.stderr));
    let stdout2 = String::from_utf8_lossy(&out2.stdout).to_string();

    assert_eq!(stdout1, stdout2, "cached scan must produce identical output");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_json_with_match_filters_entities() {
    let dir = std::env::temp_dir().join("code_seek_inttest_json_match");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }\nfn sub(a: i32, b: i32) -> i32 { a - b }",
    )
    .unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json", "--match", "add"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["total_entities"], 1);
    assert_eq!(json["files"][0]["entities"][0]["name"], "add");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_json_lang_filter_applies() {
    let dir = std::env::temp_dir().join("code_seek_inttest_json_lang");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
    fs::write(dir.join("util.c"), "int helper() { return 0; }").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json", "--lang", "rust"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["total_files"], 1);
    assert_eq!(json["files"][0]["language"], "Rust");

    fs::remove_dir_all(&dir).unwrap();
}
