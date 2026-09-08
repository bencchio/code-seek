use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn bin() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/target/debug/code-seek")
}

const INIT_REQ: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#;
const INIT_NOTIF: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

fn run_mcp(requests: &[&str]) -> Vec<serde_json::Value> {
    let mut child = Command::new(bin())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let writer = std::thread::spawn(move || {
        stdin.write_all(input.as_bytes()).ok();
    });

    let responses: Vec<serde_json::Value> = BufReader::new(stdout)
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect();

    writer.join().unwrap();
    let _ = child.wait();
    responses
}

#[test]
fn mcp_initialize_responds_with_server_info() {
    let responses = run_mcp(&[INIT_REQ]);
    assert_eq!(responses.len(), 1);
    let r = &responses[0];
    assert_eq!(r["jsonrpc"], "2.0");
    assert_eq!(r["id"], 1);
    assert_eq!(r["result"]["serverInfo"]["name"], "code-seek");
    assert!(r["result"]["protocolVersion"].as_str().is_some());
    assert!(r.get("error").is_none());
}

#[test]
fn mcp_notification_produces_no_response() {
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF]);
    // initialize → 1 response; notification → 0 responses
    assert_eq!(responses.len(), 1);
}

#[test]
fn mcp_tools_list_includes_all_tools() {
    let tools_list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, tools_list]);
    assert_eq!(responses.len(), 2);
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"scan"));
    assert!(names.contains(&"entity"));
    assert!(names.contains(&"summary"));
    assert!(names.contains(&"locate"));
}

#[test]
fn mcp_scan_tool_returns_valid_json_structure() {
    let dir = std::env::temp_dir().join("code-seek_mcp_scan");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .unwrap();

    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"scan","arguments":{{"path":"{}"}}}}}}"#,
        dir.display()
    );
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, &call]);
    assert_eq!(responses.len(), 2);

    let content = responses[1]["result"]["content"].as_array().unwrap();
    let text = content[0]["text"].as_str().unwrap();
    let scan: serde_json::Value = serde_json::from_str(text).expect("tool output must be valid JSON");

    assert_eq!(scan["total_files"], 1);
    assert_eq!(scan["total_entities"], 1);
    assert_eq!(scan["files"][0]["language"], "Rust");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mcp_scan_tool_respects_match_filter() {
    let dir = std::env::temp_dir().join("code-seek_mcp_match");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }\nfn sub(a: i32, b: i32) -> i32 { a - b }",
    )
    .unwrap();

    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"scan","arguments":{{"path":"{}","match":"add"}}}}}}"#,
        dir.display()
    );
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, &call]);
    let text = responses[1]["result"]["content"][0]["text"].as_str().unwrap();
    let scan: serde_json::Value = serde_json::from_str(text).unwrap();

    assert_eq!(scan["total_entities"], 1);
    assert_eq!(scan["files"][0]["entities"][0]["name"], "add");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mcp_unknown_tool_returns_error() {
    let call = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"noop","arguments":{}}}"#;
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, call]);
    assert_eq!(responses.len(), 2);
    assert!(responses[1].get("error").is_some(), "expected JSON-RPC error for unknown tool");
}

#[test]
fn mcp_entity_tool_returns_entity() {
    let dir = std::env::temp_dir().join("code-seek_mcp_entity");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.rs"), "fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

    let entity_path = format!("{}/lib.rs > add", dir.display());
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"entity","arguments":{{"entity_path":"{}"}}}}}}"#,
        entity_path
    );
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, &call]);
    assert_eq!(responses.len(), 2);
    assert!(responses[1].get("error").is_none());

    let text = responses[1]["result"]["content"][0]["text"].as_str().unwrap();
    let entity: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(entity["name"], "add");
    assert_eq!(entity["entity_type"], "Function");
    assert_eq!(entity["entity_path"], entity_path);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mcp_entity_tool_returns_error_for_unknown() {
    let call = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"entity","arguments":{"entity_path":"src/main.rs > nonexistent"}}}"#;
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, call]);
    assert_eq!(responses.len(), 2);
    assert!(responses[1].get("error").is_some(), "expected error for unknown entity_path");
}

#[test]
fn mcp_summary_tool_returns_structure() {
    let dir = std::env::temp_dir().join("code-seek_mcp_summary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.rs"), "fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"summary","arguments":{{"path":"{}"}}}}}}"#,
        dir.display()
    );
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, &call]);
    assert_eq!(responses.len(), 2);
    assert!(responses[1].get("error").is_none());

    let text = responses[1]["result"]["content"][0]["text"].as_str().unwrap();
    let summary: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(summary["total_files"], 1);
    assert_eq!(summary["total_entities"], 1);
    assert!(summary["by_language"].as_array().is_some_and(|a| !a.is_empty()));
    assert!(summary["by_entity_type"].as_array().is_some_and(|a| !a.is_empty()));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn mcp_locate_tool_returns_matches() {
    let dir = std::env::temp_dir().join("code-seek_mcp_locate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.rs"), "fn run() {}\nfn helper() {}").unwrap();

    let call = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"locate","arguments":{{"name":"run","path":"{}"}}}}}}"#,
        dir.display()
    );
    let responses = run_mcp(&[INIT_REQ, INIT_NOTIF, &call]);
    assert_eq!(responses.len(), 2);
    assert!(responses[1].get("error").is_none(), "unexpected error: {:?}", responses[1]);

    let text = responses[1]["result"]["content"][0]["text"].as_str().unwrap();
    let result: serde_json::Value = serde_json::from_str(text).expect("output must be valid JSON");

    assert_eq!(result["name"], "run");
    let matches = result["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["name"], "run");
    assert_eq!(matches[0]["entity_type"], "Function");
    assert!(matches[0]["entity_path"].as_str().unwrap().ends_with("> run"));

    std::fs::remove_dir_all(&dir).unwrap();
}
