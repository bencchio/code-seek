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
fn scan_json_includes_imports_field() {
    let dir = std::env::temp_dir().join("code_seek_inttest_imports");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.rs"), "use std::collections::HashMap;\nfn main() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let file = &json["files"][0];
    assert!(file.get("imports").is_some());
    let imports = file["imports"].as_array().unwrap();
    assert!(!imports.is_empty());
    assert!(imports[0].as_str().unwrap().contains("HashMap"));

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
fn scan_json_includes_dependencies_field() {
    let dir = std::env::temp_dir().join("code_seek_inttest_deps");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("main.rs"),
        "use std::collections::HashMap;\nfn main() {}",
    )
    .unwrap();
    fs::write(dir.join("helper.rs"), "pub fn helper() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let main_file = json["files"].as_array().unwrap().iter().find(|f| {
        f["path"].as_str().unwrap().ends_with("main.rs")
    }).unwrap();
    assert!(main_file.get("dependencies").is_some(), "dependencies field missing");
    let deps = main_file["dependencies"].as_array().unwrap();
    assert!(!deps.is_empty(), "dependencies should not be empty");
    let std_dep = deps.iter().find(|d| d["name"] == "std").expect("should have std dep");
    assert_eq!(std_dep["kind"], "external", "std should be external");

    let helper_file = json["files"].as_array().unwrap().iter().find(|f| {
        f["path"].as_str().unwrap().ends_with("helper.rs")
    }).unwrap();
    let helper_deps = helper_file["dependencies"].as_array().unwrap();
    assert!(helper_deps.is_empty(), "helper.rs has no imports");

    // tree output should include deps info
    let out_tree = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&out_tree.stdout);
    assert!(stdout_tree.contains("deps"), "tree output should mention dependencies");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_json_dependency_kind_internal() {
    let dir = std::env::temp_dir().join("code_seek_inttest_deps_internal");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    // main.rs imports std (external) and helper (internal, file exists in scan root)
    fs::write(
        dir.join("src/main.rs"),
        "use std::collections::HashMap;\nuse helper;\nfn main() {}",
    )
    .unwrap();
    fs::write(dir.join("src/helper.rs"), "pub fn help() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let main_file = json["files"].as_array().unwrap().iter().find(|f| {
        f["path"].as_str().unwrap().ends_with("main.rs")
    }).unwrap();
    let deps = main_file["dependencies"].as_array().unwrap();

    let std_dep = deps.iter().find(|d| d["name"] == "std").expect("should have std dep");
    assert_eq!(std_dep["kind"], "external", "std should be external");

    let helper_dep = deps.iter().find(|d| d["name"] == "helper").expect("should have helper dep");
    assert_eq!(helper_dep["kind"], "internal", "helper should be internal");

    // tree output should show both ext and int deps
    let out_tree = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout_tree = String::from_utf8_lossy(&out_tree.stdout);
    assert!(stdout_tree.contains("ext deps"), "tree should show ext deps count");
    assert!(stdout_tree.contains("int deps"), "tree should show int deps count");

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

#[test]
fn scan_json_includes_errors_field() {
    let dir = std::env::temp_dir().join("code_seek_inttest_errors");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // File with syntax error — missing expression after '='
    fs::write(dir.join("bad.rs"), "fn main() { let x = }\n").unwrap();
    // Valid file — no errors
    fs::write(dir.join("ok.rs"), "fn ok() {}\n").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // top-level total_errors — single-line input produces exactly 1 error
    assert_eq!(json["total_errors"].as_u64().unwrap(), 1, "total_errors should be exactly 1");

    // bad.rs has errors with correct field types and values
    let bad = json["files"].as_array().unwrap().iter()
        .find(|f| f["path"].as_str().unwrap().ends_with("bad.rs"))
        .unwrap();
    let errors = bad["errors"].as_array().unwrap();
    assert!(!errors.is_empty(), "bad.rs should have errors");
    let e = &errors[0];
    assert!(["error", "missing"].contains(&e["kind"].as_str().unwrap()), "kind must be error or missing");
    assert!(e["node_kind"].as_str().is_some(), "node_kind must be a string");
    assert!(e["start_line"].as_u64().is_some(), "start_line must be an integer");
    assert!(e["end_line"].as_u64().is_some(), "end_line must be an integer");
    assert!(e["start_line"].as_u64().unwrap() >= 1, "start_line must be >= 1");

    // ok.rs has no errors
    let ok = json["files"].as_array().unwrap().iter()
        .find(|f| f["path"].as_str().unwrap().ends_with("ok.rs"))
        .unwrap();
    assert_eq!(ok["errors"].as_array().unwrap().len(), 0, "ok.rs should have no errors");

    // tree output: bad.rs header shows error count, main has ⚠, ok has ✓
    let out_tree = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out_tree.status.success());
    let stdout_tree = String::from_utf8_lossy(&out_tree.stdout);
    assert!(stdout_tree.contains("1 error"), "bad.rs header should show '1 error'");
    assert!(stdout_tree.contains('⚠'), "bad.rs entity line should have ⚠ marker");
    assert!(stdout_tree.contains('✓'), "ok.rs entity line should have ✓ marker");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_with_ignore_flag_skips_dirs() {
    let dir = std::env::temp_dir().join("code_seek_inttest_ignore");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("target/debug")).unwrap();
    fs::write(dir.join("src/main.rs"), b"fn main() {}").unwrap();
    fs::write(dir.join("target/debug/build.rs"), b"fn build() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--ignore", "target"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("main.rs"), "src/main.rs must appear");
    assert!(!stdout.contains("build.rs"), "target/debug/build.rs must be skipped");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_empty_rs_file() {
    let dir = std::env::temp_dir().join("code_seek_inttest_empty_rs");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("empty.rs"), b"").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("0 entities"), "empty file must show 0 entities");
    assert!(stdout.contains("0 LOC"), "empty file must show 0 LOC");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_only_comments_rs() {
    let dir = std::env::temp_dir().join("code_seek_inttest_comments_rs");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("comments.rs"), b"// this is a comment\n// another line\n").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("0 entities"), "comment-only file must show 0 entities");

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_invalid_lang_cli() {
    let dir = std::env::temp_dir().join("code_seek_inttest_invalid_lang");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("main.rs"), b"fn main() {}").unwrap();

    let out = Command::new(bin())
        .args(["scan", dir.to_str().unwrap(), "--lang", "python"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "invalid lang must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("python"), "error must mention the invalid lang");

    fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn scan_symlink_root_no_follow() {
    use std::os::unix::fs::symlink;
    let tmp = std::env::temp_dir().join("code_seek_inttest_symlink_root");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("real")).unwrap();
    fs::write(tmp.join("real/main.rs"), b"fn main() {}").unwrap();
    symlink(tmp.join("real"), tmp.join("link")).unwrap();

    let out = Command::new(bin())
        .args(["scan", tmp.join("link").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("0 files"), "symlink root without follow must show 0 files");

    fs::remove_dir_all(&tmp).unwrap();
}
