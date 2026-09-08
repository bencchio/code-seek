# Code Seek — Checklist

Completed increments per cycle. Items here are done and should not appear in the Roadmap.

## Cycle 0.3.x — Dependencies & robustness

| Version | Date | Increment |
| ------- | ---- | --------- |
| 0.3.0   | 2026-07-02 | Parser refactor: removed `LanguageParser` trait and per-language structs; `parse_impl` free functions; depth limit (64) in `collect_children`; doc reorganization |
| 0.3.1   | 2026-07-02 | Import detection: `imports` field in JSON output (`#include` C/C++, `use`/`extern crate` Rust, `import` QML); `parse_all` single-parse pattern; `LocMap` extracted to `lang/loc.rs`; `scan_one` / `apply_filters` extracted |
| 0.3.2   | 2026-07-02 | Dependency graph: `dependencies` field in JSON output classifying each import as `internal` (resolves to a file in the scan root) or `external` (third-party / stdlib) |
| 0.3.3   | 2026-07-02 | Syntax error detection: `errors` field per file + `total_errors` top-level in JSON; tree header shows error count; `detect_syntax_errors` helper (no recursion into ERROR children); errors cached |
| 0.3.4  | 2026-07-02 | Stabilization: `--ignore` CLI/MCP flag, dead code removal (visibility narrowing), coverage >80% (9 new unit tests), edge case hardening (5 new integration tests), last production panic eliminated |
| 0.4.0   | 2026-07-02 | Release — closes 0.3.x cycle. First public release candidate |

## Cycle 0.2.x — AI-ready

| Version | Date | Increment |
| ------- | ---- | --------- |
| 0.2.0   | 2026-06-30 | Release — closes 0.1.x MVP cycle |
| 0.2.1   | 2026-06-30 | Refactor: `Entity::new`, `parse_source`, `collect_children`, `LangDef` registry |
| 0.2.2   | 2026-06-30 | `--format json`: structured JSON output, `entity_path` field |
| 0.2.3   | 2026-06-30 | `--match` and `--max-depth` entity filters |
| 0.2.4   | 2026-06-30 | `code-seek mcp`: JSON-RPC 2.0 stdio server, `scan` tool |
| 0.2.5   | 2026-06-30 | MCP `entity`, `summary` tools; structured stderr logging; path traversal protection |
| 0.2.6   | 2026-07-01 | SHA-256 incremental cache (`.code-seek/cache.json`) |
| 0.2.7   | 2026-07-01 | MCP `locate` tool: case-insensitive entity name search |
