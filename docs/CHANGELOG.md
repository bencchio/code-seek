# Changelog

All notable changes to this project are documented in this file.

## [0.3.0] — 2026-07-02

### Changed
- Parser helpers shared across languages; docs moved under reference, rules, and design.

## [0.2.7] — 2026-07-01

### Added
- MCP `locate` tool: given an entity `name` and a `path` to scan, returns all matching entities across all files — with `entity_path`, `entity_type`, `loc`, `start_line`, `end_line`; search is case-insensitive; optional `lang` filter supported
- `scan::locate(results, name)`: walks the full entity tree across all `FileResult`s; private `collect_matches` helper builds `entity_path` recursively using the same `entity_path_segment` logic as `json_entities`
- 4 unit tests in `scan::tests`: match across files, case-insensitivity, no-match, nested entity path
- 1 integration test in `tests/mcp.rs` (`mcp_locate_tool_returns_matches`): verifies name, entity_type, and entity_path in the response
- `locate_tool_def()` in `mcp.rs`; `locate` registered in `tools/list` (now 4 tools) and `handle_tools_call`

## [0.2.6] — 2026-07-01

### Added
- `src/cache.rs`: SHA256-based incremental scan cache; stores `sha256`, `language`, `loc`, and `entities` per file in `.code-seek/cache.json`
- `cache::sha256(content)`: computes SHA-256 hex digest of file content using the `sha2` crate
- `cache::load(path)`: reads `.code-seek/cache.json`; silently returns an empty cache when the file does not exist; emits `[WARN]` and returns empty on corrupt JSON
- `cache::Cache::get(key, sha256)`: cache hit only when key exists **and** hash matches; reconstructs `&'static str` language from `lang::LANGUAGES` registry; returns `None` on miss or unknown language
- `cache::Cache::insert(key, result, sha256)`: upserts a file result into the cache
- `cache::Cache::save(path)`: writes the cache as compact JSON; skips write silently when the parent directory (`.code-seek/`) does not exist; emits `[WARN]` on write failure — never aborts
- `scan::build_scan_results`: loads cache before the file loop; skips `lang::parse_file` on cache hit; writes cache after the loop only when at least one file was (re-)parsed (`cache_dirty` flag)
- 4 unit tests in `src/cache.rs`: miss, hash mismatch, hit reconstruction, save-and-load roundtrip
- 1 integration test in `tests/scan.rs` (`scan_uses_cache_on_second_run`): verifies `cache.json` is written after first scan and that a second scan produces identical output

### Changed
- `model::EntityType` and `model::Entity`: added `Clone`, `serde::Serialize`, `serde::Deserialize` derives (required for cache serialization)
- `Cargo.toml`: added `sha2 = "0.10"` dependency; bumped version to `0.2.6`

## [0.2.5] — 2026-07-01

### Added
- `src/log.rs`: structured stderr logging module with `info`, `warn`, and `error` functions; all log output is prefixed with `[INFO]`, `[WARN]`, or `[ERROR]`
- MCP `initialize`: `protocolVersion` is now negotiated from the client request — server echoes the client's version when supported, falls back to `"2024-11-05"` otherwise
- MCP errors now return a proper JSON-RPC `error` field (`{"code": ..., "message": "..."}`) instead of `{"result": {"isError": true, ...}}`
- MCP request logging: every incoming method is logged to stderr via `[INFO]`; invalid JSON emits `[WARN]`; scan/entity errors emit `[ERROR]`
- MCP `entity` tool: looks up a single entity by its `entity_path` string (e.g. `"src/main.rs > impl Foo > bar"`); scans only the referenced file; returns JSON record with `name`, `entity_type`, `loc`, `start_line`, `end_line`, `children`; returns JSON-RPC error `-32000` when not found
- MCP `summary` tool: returns aggregate counts for a `path` — `total_files`, `total_loc`, `total_entities`, `by_language` (alphabetical), `by_entity_type` (alphabetical); accepts optional `lang` filter
- `scan::find_entity(results, entity_path)`: walks the entity tree segment-by-segment via `entity_path_segment`; returns `serde_json::Value` or `None`
- `scan::summarize(results, scan_path)`: aggregates counts using `BTreeMap` for deterministic ordering; private `count_by_type` helper
- Path traversal validation (`..` components) in all three MCP tools; empty `entity_path` without `" > "` separator rejected with `-32602`

### Changed
- `scan.rs`: oversized-file and unreadable-file warnings use `crate::log::warn`
- `mcp.rs`: `handle_tools_call` dispatches via `match` to `handle_scan`, `handle_entity`, `handle_summary`; `tools/list` exposes all three tools; MCP tool responses use compact JSON (not pretty-printed)

## [0.2.4] — 2026-06-30

### Added
- `code-seek mcp`: MCP server over stdio; implements JSON-RPC 2.0 `initialize`, `tools/list`, and `tools/call` with a single `scan` tool
- `scan` MCP tool: same output schema as `--format json`; supports `path`, `lang`, `match`, `max_depth` arguments
- `scan::build_scan_results()`: extracted from `scan::run()` — returns `Vec<FileResult>` without printing; used by both CLI and MCP server
- `scan::results_to_json()`: extracted from `print_json()` — returns `serde_json::Value`; used by both CLI and MCP server
- `docs/MCP.md`: setup instructions for Claude Code and OpenCode, tool reference, raw JSON-RPC session example
- 6 integration tests in `tests/mcp.rs`: initialize handshake, notification skipping, tools/list, scan tool output, match filter, unknown tool error

## [0.2.3] — 2026-06-30

### Added
- `--match <PATTERN>` flag for `code-seek scan`: case-insensitive substring filter on entity name; parents are preserved when a child matches; files with zero matching entities still show their header
- `--max-depth <N>` flag for `code-seek scan`: hard depth ceiling — `0` = file headers only, `1` = root entities, `2` = root + direct children; no flag = unlimited (current behavior)
- Both flags compose: depth is a hard ceiling, match filters within the depth-limited result
- 5 unit tests for `filter_entities` (depth-0 empty, depth-1 roots, match exclusion, parent preservation, depth-wins-over-match)
- 5 integration tests (match filters, parent preservation, depth truncation, zero-match header, combined flags)

### Changed
- `scan::run` accepts two additional parameters: `match_pattern: &str` and `max_depth: Option<usize>`

## [0.2.2] — 2026-06-30

### Added
- `--format json` flag for `code-seek scan`: structured JSON output consumable by AI tools and scripts; tree format remains the default
- `entity_path` field in JSON output — canonical unique identifier per entity following the format `path > [parent >]* name`; impl/trait/mod container entities are prefixed (`impl Foo`, `trait Bar`, `mod utils`)
- JSON output schema fields: `version`, `scan_path`, `total_files`, `total_loc`, `total_entities`, `files[].{path,language,loc,entity_count,entities}`
- 3 integration tests for JSON format (valid JSON output, entity path correctness, `--lang` filter compatibility)
- 3 unit tests for `entity_path_segment` and `entity_type_str` helpers

### Changed
- `scan::run` accepts a third `format: &str` parameter; tree output extracted to `print_tree()` for clarity

## [0.2.1] — 2026-06-30

Refactor — eliminates duplication across language parsers; no behavior change.

### Added
- `Entity::new()` constructor for leaf entities (children always empty); simplifies parser code
- `parse_source()` helper in `lang/mod.rs`: centralizes tree-sitter parser initialization with `.expect()` instead of `.unwrap()`
- `collect_children()` helper in `lang/mod.rs`: generic iterator replacing 3 × identical `parse_nodes()` functions in c.rs, cpp.rs, rust.rs
- `save_history_by_default: bool` field added to `ScanConfig` (default: `false`); now correctly parsed from config instead of silently dropped
- `LangDef` struct and `LANGUAGES` static registry in `lang/mod.rs`: single source of truth for canonical name, file extensions, `--lang` aliases, and Nerd Font icon; adding a new language now requires only one registry entry and one parser module
- 4 unit tests for `config::ScanConfig` deserialization (defaults, valid TOML, `save_history_by_default`, invalid TOML)
- 3 integration tests for `code-seek init`: happy path, already-exists error, `--force` overwrite

### Changed
- All `pub` items narrowed to `pub(crate)` across all modules (model.rs, config.rs, lang/mod.rs, lang/*.rs, scan.rs, init.rs, walker.rs)
- `impl_name` renamed to `impl_target_type` in rust.rs — name now describes what the function returns
- `parse_nodes()` removed from c.rs, cpp.rs, rust.rs — replaced by `collect_children()` from lang/mod.rs
- Parser boilerplate removed from c.rs, cpp.rs, rust.rs, qml.rs — replaced by `parse_source()` from lang/mod.rs
- Dead `in_class` parameter removed from `parse_node` in c.rs — C has no class context; `EntityType::Method` was unreachable
- Spelling: `unrecognised` → `unrecognized` in `lang/mod.rs` `unreachable!` message
- `detect()` and `parse_file()` in `lang/mod.rs` now derive from `LANGUAGES` instead of parallel match arms
- `lang_matches()` and `lang_icon()` in `scan.rs` now derive from `LANGUAGES` instead of hardcoded string tables

## [0.2.0] — 2026-06-30

Release — closes the 0.1.x MVP cycle.

### Changed
- `EntityType`: removed unused `Clone` and `Eq` derives — only `Debug` and `PartialEq` remain
- `Entity`: removed unused `Clone` derive — only `Debug` remains
- `LanguageParser`: narrowed visibility from `pub` to `pub(crate)` (binary crate, no external consumers)
- `parse_file`: replaced unreachable `_ => Vec::new()` arm with `unreachable!()` to communicate the invariant

## [0.1.99] — 2026-06-30

### Added
- `EntityType::Impl` and `EntityType::Trait` — Rust `impl` blocks and `trait` definitions now map to their own entity types instead of `Class`; icons: 󰉺 Impl, 󰜁 Trait
- `follow_symlinks` field in `ScanConfig` (default: `false`): walker skips symlinks unless explicitly enabled
- `max_file_size_mb` field in `ScanConfig` (default: `10.0`): files exceeding the limit are skipped with a stderr warning
- `LocMap`: prefix-sum structure that pre-computes LOC per line once per file (O(n) build, O(1) per entity), replacing the previous O(n²) `count_loc_range` calls inside each parser
- Warning messages to stderr for unreadable files and oversized files; scan never silently discards errors
- 6 new tests: `LocMap` unit tests (5) and `follow_symlinks = false` walker test

### Changed
- Walker accepts `follow_symlinks: bool` parameter; symlinks (files and dirs) are skipped by default
- `scan.rs` no longer accumulates intermediate `FileBlock`/`EntityRow` representations; prints directly from `Vec<FileResult>` with global column alignment computed in a single tree walk
- `config::ScanConfig` now exposes `follow_symlinks` and `max_file_size_mb` (parsed from `.code-seek/config.toml`; defaults apply when fields are absent)

### Fixed
- Rust `impl Trait for Type` → reported as `EntityType::Impl` with the implementing type as name (was `EntityType::Class`)
- Rust `trait Foo` → reported as `EntityType::Trait` (was `EntityType::Class`)
- Silent swallow of file-read errors replaced with stderr warning

## [0.1.5] — 2026-06-30

### Added
- `code-seek scan --lang <langs>` flag: filter output by language (c, cpp, qml, rust); comma-separated for multiple (`--lang qml,cpp`)
- `ignore_dirs` in `.code-seek/config.toml`: walker skips listed directories; `code-seek init` pre-fills with common build/dependency dirs for all supported ecosystems
- `docs/DEPENDENCIES.md`: lists all runtime crates with version and purpose

### Changed
- Walker now returns only files with supported extensions; unsupported files are silently skipped
- `code-seek init` config template extended with comprehensive `ignore_dirs` defaults (target, build, node_modules, dist, __pycache__, vendor, Pods, etc.)
- Config loaded from CWD (`.code-seek/config.toml`) instead of scan root, so one `code-seek init` covers scans of any path

## [0.1.4] — 2026-06-30

### Added
- QML parser: root object → Class (id as name, type as fallback), function declarations → Method (tree-sitter-qmljs)
- Nested objects and property bindings intentionally skipped; only function declarations captured

### Changed
- Upgraded tree-sitter 0.22 → 0.25 and all parser crates to latest compatible versions
- Parser language registration updated from `language()` to `LANGUAGE.into()` (new tree-sitter API)

## [0.1.3] — 2026-06-30

### Added
- Rust parser: fn, struct, impl, trait, enum, mod (tree-sitter-rust)
- `impl Foo` → Class entity with Method children; `trait Foo` → Class entity with Method children
- `mod foo { }` → Namespace entity; `mod foo;` (no body) skipped
- Global column alignment across all files in a scan (col, LOC width, line-number width)
- LOC right-aligned and `[start-end]` internally padded for consistent columns

## [0.1.2] — 2026-06-30

### Added
- Entity model: EntityType, Entity, FileResult
- C parser: functions, structs, enums (tree-sitter-c)
- C++ parser: functions, classes, structs, enums, namespaces, methods (tree-sitter-cpp)
- code-seek scan reports LOC and entity count per file with tree output and Nerd Font icons
- Stateful block-comment LOC counting (no spurious inflation from `/* ... */` bodies)
- Walker handles single-file paths in addition to directories

### Fixed
- `parser.parse()` returning `None` on deeply nested code no longer panics

## [0.1.1] — 2026-06-30
### Added
- Tree-sitter dependency (v0.22)
- Language detection by extension: C (.c .h), C++ (.cpp .cc .cxx .hpp .hxx), Rust (.rs), QML (.qml)
- code-seek scan annotates each file with its detected language

## [0.1.0] — 2026-06-30

### Added

- Project scaffold with cargo init
- CLI skeleton with clap
- code-seek init: creates .code-seek/ with default config.toml
- File walker with hardcoded .git/ and .code-seek/ ignore
- Initial documentation (SPECS, BACKLOG, AGENTS, README, CHANGELOG)
