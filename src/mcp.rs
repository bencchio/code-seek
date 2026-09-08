use crate::scan;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

const SUPPORTED_VERSIONS: &[&str] = &["2024-11-05"];
const DEFAULT_VERSION: &str = "2024-11-05";

fn negotiate_version(msg: &Value) -> &'static str {
    let client = msg["params"]["protocolVersion"].as_str().unwrap_or("");
    SUPPORTED_VERSIONS.iter().copied().find(|&v| v == client).unwrap_or(DEFAULT_VERSION)
}

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                crate::log::warn("received invalid JSON, skipping");
                continue;
            }
        };
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("?");
        // Notifications have no "id" field — no response required.
        let Some(id) = msg.get("id") else {
            crate::log::info(&format!("→ {method} (notification)"));
            continue;
        };
        let id = id.clone();
        crate::log::info(&format!("→ {method}"));

        let response = dispatch(id, &msg);
        writeln!(out, "{}", serde_json::to_string(&response)?)?;
        out.flush()?;
    }
    Ok(())
}

fn dispatch(id: Value, msg: &Value) -> Value {
    match msg.get("method").and_then(|m| m.as_str()).unwrap_or("") {
        "initialize" => json!({
            "jsonrpc": "2.0", "id": id,
            "result": {
                "protocolVersion": negotiate_version(msg),
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "code-seek",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0", "id": id,
            "result": {"tools": [scan_tool_def(), entity_tool_def(), summary_tool_def(), locate_tool_def()]}
        }),
        "tools/call" => handle_tools_call(id, msg),
        _ => json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32601, "message": "Method not found"}
        }),
    }
}

fn handle_tools_call(id: Value, msg: &Value) -> Value {
    let params = &msg["params"];
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "scan"    => handle_scan(id, &args),
        "entity"  => handle_entity(id, &args),
        "summary" => handle_summary(id, &args),
        "locate"  => handle_locate(id, &args),
        _ => json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": format!("Unknown tool: {tool_name}")}
        }),
    }
}

fn has_traversal(path_str: &str) -> bool {
    std::path::Path::new(path_str)
        .components()
        .any(|c| c == std::path::Component::ParentDir)
}

fn validate_mcp_path(path_str: &str) -> Result<std::path::PathBuf, String> {
    if has_traversal(path_str) {
        return Err("path traversal not allowed".to_owned());
    }
    let path = std::path::Path::new(path_str);
    path.canonicalize().map_err(|e| format!("invalid path: {e}"))
}

fn handle_scan(id: Value, args: &Value) -> Value {
    let Some(path_str) = args.get("path").and_then(|p| p.as_str()) else {
        return json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": "Missing required argument: path"}
        });
    };
    let path = match validate_mcp_path(path_str) {
        Ok(p) => p,
        Err(e) => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": e}}),
    };

    let lang_filter: Vec<String> = args
        .get("lang")
        .and_then(|l| l.as_str())
        .map(|s| s.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();
    let match_pattern = args.get("match").and_then(|m| m.as_str()).unwrap_or("");
    let max_depth = args.get("max_depth").and_then(|d| d.as_u64()).map(|d| d as usize);
    let extra_ignore: Vec<String> = args
        .get("ignore")
        .and_then(|i| i.as_str())
        .map(|s| s.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();
    let info = args.get("info").and_then(|i| i.as_str()).unwrap_or("all");

    match scan::build_scan_results(&path, &lang_filter, match_pattern, max_depth, &extra_ignore, info) {
        Ok(results) => {
            let text = match serde_json::to_string(&scan::results_to_json(&results, &path)) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("internal error: {e}");
                    return json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32603, "message": msg}
                    });
                }
            };
            json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}})
        }
        Err(e) => {
            crate::log::error(&e.to_string());
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": e.to_string()}})
        }
    }
}

fn handle_entity(id: Value, args: &Value) -> Value {
    let Some(entity_path) = args.get("entity_path").and_then(|p| p.as_str()) else {
        return json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": "Missing required argument: entity_path"}
        });
    };
    if !entity_path.contains(" > ") {
        return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": "entity_path must contain at least one ' > ' separator"}});
    }

    let file_path = entity_path.split(" > ").next().unwrap_or("");
    let path = match validate_mcp_path(file_path) {
        Ok(p) => p,
        Err(e) => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": e}}),
    };

    match scan::build_scan_results(&path, &[], "", None, &[], "all") {
        Ok(results) => match scan::find_entity(&results, entity_path) {
            Some(v) => {
                let text = match serde_json::to_string(&v) {
                    Ok(s) => s,
                    Err(e) => {
                        let msg = format!("internal error: {e}");
                        return json!({
                            "jsonrpc": "2.0", "id": id,
                            "error": {"code": -32603, "message": msg}
                        });
                    }
                };
                json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}})
            }
            None => {
                crate::log::error(&format!("entity not found: {entity_path}"));
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": format!("entity not found: {entity_path}")}})
            }
        },
        Err(e) => {
            crate::log::error(&e.to_string());
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": e.to_string()}})
        }
    }
}

fn handle_summary(id: Value, args: &Value) -> Value {
    let Some(path_str) = args.get("path").and_then(|p| p.as_str()) else {
        return json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": "Missing required argument: path"}
        });
    };
    let path = match validate_mcp_path(path_str) {
        Ok(p) => p,
        Err(e) => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": e}}),
    };

    let lang_filter: Vec<String> = args
        .get("lang")
        .and_then(|l| l.as_str())
        .map(|s| s.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();

    match scan::build_scan_results(&path, &lang_filter, "", None, &[], "all") {
        Ok(results) => {
            let text = match serde_json::to_string(&scan::summarize(&results, &path)) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("internal error: {e}");
                    return json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32603, "message": msg}
                    });
                }
            };
            json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}})
        }
        Err(e) => {
            crate::log::error(&e.to_string());
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": e.to_string()}})
        }
    }
}

fn handle_locate(id: Value, args: &Value) -> Value {
    let Some(name) = args.get("name").and_then(|n| n.as_str()) else {
        return json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": "Missing required argument: name"}
        });
    };
    let Some(path_str) = args.get("path").and_then(|p| p.as_str()) else {
        return json!({
            "jsonrpc": "2.0", "id": id,
            "error": {"code": -32602, "message": "Missing required argument: path"}
        });
    };
    let path = match validate_mcp_path(path_str) {
        Ok(p) => p,
        Err(e) => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": e}}),
    };

    let lang_filter: Vec<String> = args
        .get("lang")
        .and_then(|l| l.as_str())
        .map(|s| s.split(',').map(str::trim).map(str::to_owned).collect())
        .unwrap_or_default();

    match scan::build_scan_results(&path, &lang_filter, "", None, &[], "all") {
        Ok(results) => {
            let matches = scan::locate(&results, name);
            let text = match serde_json::to_string(&serde_json::json!({
                "name": name,
                "matches": matches,
            })) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("internal error: {e}");
                    return json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32603, "message": msg}
                    });
                }
            };
            json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}})
        }
        Err(e) => {
            crate::log::error(&e.to_string());
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": e.to_string()}})
        }
    }
}

fn lang_filter_description() -> String {
    let aliases: Vec<&str> = crate::lang::LANGUAGES
        .iter()
        .flat_map(|l| l.aliases.iter().copied())
        .collect();
    format!("Comma-separated language filter: {}", aliases.join(", "))
}

fn scan_tool_def() -> Value {
    json!({
        "name": "scan",
        "description": "Scan a directory or file and return a structured entity index — functions, methods, classes, structs, traits, enums, namespaces — with LOC and line numbers. Use this to navigate code or detect refactor opportunities without reading raw source files.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory or file path to scan"
                },
                "lang": {
                    "type": "string",
                    "description": lang_filter_description()
                },
                "match": {
                    "type": "string",
                    "description": "Case-insensitive substring filter on entity name; parents with matching children are preserved"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Hard depth ceiling: 0 = file headers only, 1 = root entities, 2 = roots + direct children"
                },
                "ignore": {
                    "type": "string",
                    "description": "Comma-separated directory names to skip (e.g. \"target,node_modules\")"
                },
                "info": {
                    "type": "string",
                    "description": "Entity info filter: \"all\" (default), \"no-tests\" (exclude test modules), \"tests-only\" (show only test modules)"
                }
            },
            "required": ["path"]
        }
    })
}

fn entity_tool_def() -> Value {
    json!({
        "name": "entity",
        "description": "Look up a single entity by its entity_path and return its full JSON record (name, type, LOC, line range, children). Scans only the file referenced in the entity_path.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "entity_path": {
                    "type": "string",
                    "description": "Full entity path as produced by the scan tool, e.g. \"src/main.rs > impl Foo > bar\""
                }
            },
            "required": ["entity_path"]
        }
    })
}

fn locate_tool_def() -> Value {
    json!({
        "name": "locate",
        "description": "Search for an entity by name across a directory and return all matches with their entity_path and exact line numbers. Use this when you know the entity name but not which file it lives in.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Entity name to search for (case-insensitive)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file path to search in"
                },
                "lang": {
                    "type": "string",
                    "description": lang_filter_description()
                }
            },
            "required": ["name", "path"]
        }
    })
}

fn summary_tool_def() -> Value {
    json!({
        "name": "summary",
        "description": "Return aggregate counts for a path: total files, LOC, and entities, broken down by language and entity type.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory or file path to scan"
                },
                "lang": {
                    "type": "string",
                    "description": lang_filter_description()
                }
            },
            "required": ["path"]
        }
    })
}
