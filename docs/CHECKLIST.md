# Code Seek — Checklist

Completed increments per cycle. Items here are done and should not appear in the Roadmap.

## Cycle 0.4.x — Fixes, Language Expansion

| Version | Date | Increment |
| ------- | ---- | --------- |
| 0.4.2   | 2026-07-03 | JavaScript parser (`lang/js.rs`): Function, Class, Method, arrow functions, export wrappers, ES imports; MCP install/troubleshooting docs |
| 0.4.3   | 2026-07-03 | Python parser (`lang/python.rs`): Function, Class, Method, async/decorated definitions, `import`/`from … import` resolution |
| 0.4.4   | 2026-07-07 | TypeScript parser (`lang/ts.rs`): JS constructs + interface→Trait, Enum, namespace→Namespace, abstract classes; `.ts`/`.mts`/`.cts` |
| 0.4.5   | 2026-07-07 | Language infrastructure refactor: `LangDef`-driven generic parse pipeline, JS/TS and C/C++ unification, entity helpers, `Context`/`ParseOutput`; MCP `lang` filter description generated from registry |
| 0.4.6   | 2026-07-10 | Go parser (`lang/go.rs`): Function, Method, Struct, Trait (interface), `import_declaration` with grouped/alias support |
| 0.4.99  | 2026-07-10 | Stabilization: 5 new filter integration tests, 3 new Go unit tests, duplicate test removed, let-chain refactor, docs completed for Go |

## Cycle 0.5.x — Security, Documentation & Roadmap

| Version | Date | Increment |
| ------- | ---- | --------- |
| 0.5.0   | 2026-07-14 | Security audit: MCP path traversal protection, cache symlink attack prevention, TOCTOU fixes, JSON-RPC serialization error handling, `main() -> Result`; docs: updated README/SPECS/ROADMAP/CHANGELOG; roadmap restructured into 0.5.x (CLI), 0.6.x (agent tools), post-0.6.x |
| 0.5.1   | 2026-09-07 | Rename project to Code Seek: crate/CLI/MCP `code-seek`, state dir `.code-seek/` |
| 0.5.2   | 2026-09-08 | XDG state: config and cache under `~/.local/state/code-seek/<basename>/`; no in-repo `.code-seek/` |
| 0.5.3   | 2026-09-08 | Elixir parser (`.ex`/`.exs`): defmodule, def/defp, macros, protocol, impl, struct; entity `kind` |
| 0.5.4   | 2026-09-08 | GitHub-ready main: MIT license, public readme, `main` rebuilt sanitized, git hooks activated |
| 0.5.5   | 2026-09-08 | Packaging: `install.sh`, AUR `PKGBUILD`, crate metadata; scan honors `.gitignore` with `--no-gitignore` opt-out |
| 0.6.0   | 2026-09-08 | Release — closes the 0.5.x cycle. Public history published with bare-version tags; release script attaching the Arch package |

## Cycle 0.3.x — Dependencies & robustness

| Version | Date | Increment |
| ------- | ---- | --------- |
| 0.3.0   | 2026-07-02 | Parser refactor: removed `LanguageParser` trait and per-language structs; `parse_impl` free functions; depth limit (64) in `collect_children`; doc reorganization |
| 0.3.1   | 2026-07-02 | Import detection: `imports` field in JSON output (`#include` C/C++, `use`/`extern crate` Rust, `import` QML); `parse_all` single-parse pattern; `LocMap` extracted to `lang/loc.rs`; `scan_one` / `apply_filters` extracted |
| 0.3.2   | 2026-07-02 | Dependency graph: `dependencies` field in JSON output classifying each import as `internal` (resolves to a file in the scan root) or `external` (third-party / stdlib) |
| 0.3.3   | 2026-07-02 | Syntax error detection: `errors` field per file + `total_errors` top-level in JSON; tree header shows error count; `detect_syntax_errors` helper (no recursion into ERROR children); errors cached |
| 0.3.99  | 2026-07-02 | Stabilization: `--ignore` CLI/MCP flag, dead code removal (visibility narrowing), coverage >80% (9 new unit tests), edge case hardening (5 new integration tests), last production panic eliminated |
| 0.4.0   | 2026-07-02 | Release — closes 0.3.x cycle. First public release candidate |
| 0.4.1   | 2026-07-03 | Security hardening & critical bug fixes: removed broken `verify_internal_deps`; fixed QML import parsing with `as X` aliases; symlink cycle detection; cache size cap (100 MB); `entities.len()-1` panic fix; non-UTF-8 import warning; `--info` filter (all/no-tests/tests-only); config default robustness fix |

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
