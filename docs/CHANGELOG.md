# Changelog

All notable changes to this project are documented in this file.

## [0.5.0] — 2026-07-14

Security audit, documentation, and roadmap 0.5.

### Security

- **MCP path traversal protection** (`src/mcp.rs`): all four MCP tools (`scan`, `entity`, `summary`, `locate`) now canonicalize user-provided paths and reject `..` traversal. Paths that fail canonicalization return a clear JSON-RPC error instead of proceeding.
- **Cache symlink attack prevention** (`src/cache.rs`): `resolve_cache_path` now returns `Option<PathBuf>`; when `.code-seek/cache.json` is a symlink pointing outside the project directory, the cache is skipped entirely (both load and save). Previously only emitted a warning but still followed the symlink.
- **TOCTOU in file size check** (`src/scan/mod.rs`): file size is now verified against the already-loaded content instead of a pre-read `fs::metadata()` call, eliminating the race window where a file could be swapped between size check and read.
- **JSON-RPC serialization failures** (`src/mcp.rs`): `serde_json::to_string` failures now return a proper JSON-RPC error (`-32603`) with the error message, instead of silently returning an empty string.

### Changed

- **`main() -> Result`** (`src/main.rs`): CLI entry point now returns `Result<(), Box<dyn std::error::Error>>` instead of calling `process::exit()` directly.
- **`walker.rs` warning logging** (`src/walker.rs`): `walk_dir` now uses `crate::log::warn` for unreadable directory warnings instead of a bare `eprintln!`, consistent with all other modules.
- **`unicode-width`** (`src/scan/render.rs`): `char_width` now uses `UnicodeWidthStr::width` instead of `chars().count()` for correct terminal column alignment with CJK characters and emoji in file paths and entity names.
- **`init` config template** (`src/init.rs`): removed the vestigial `version = "0.1"` field from the generated `.code-seek/config.toml`; the `Config` struct has no `version` field and was silently ignoring it.
- **`model.rs` structure** (`src/model.rs`): moved `impl Entity` and `FileResult` before `#[cfg(test)]` so all types are defined before the test module that uses them.

### Documentation

- **`docs/ROADMAP.md`**: restructured into three cycles — 0.5.x (CLI & change visualization), 0.6.x (agent tools), and post-0.6.x (advanced analysis & platform). Each cycle has 5 planned features.
- **`README.md`**: status updated to v0.5.0; Go added to supported languages list.
- **`docs/reference/SPECS.md`**: version bumped to 0.5.0; planned sections realigned.
- **`docs/CHECKLIST.md`**: cycle 0.5.x entry added.

## [0.4.99] — 2026-07-10

Stabilization — closes the 0.4.x cycle.

### Added

- **5 new integration tests** (`tests/scan.rs`): `filter_combined_match_and_depth`, `filter_empty_match_shows_file_header`, `filter_match_includes_matching_excludes_others`, `filter_match_preserves_parent_impl`, `filter_max_depth_truncates_children` — cover all `--match`/`--max-depth` combinations end-to-end via the compiled binary.
- **3 new Go unit tests** (`src/lang/go.rs`): `parses_only_package` (package-only source returns no entities), `skips_type_alias` (`type X = string` is not emitted), `detect_go` in `src/lang/mod.rs`.

### Fixed

- **Duplicate test removed** (`src/lang/go.rs`): `parses_struct_with_fields` was identical to `parses_struct`; removed.

### Changed

- **Let-chain refactoring** (`src/cache.rs`, `src/lang/go.rs`): nested `if let` + `if` patterns rewritten as Rust-2024 let-chains (`&&`); no behavior change.
- **Test order** (`src/lang/go.rs`, `src/lang/mod.rs`): unit tests sorted alphabetically within their `mod tests` block.

### Documentation

- `docs/reference/SPECS.md`: version bumped to 0.4.99; Go added to supported-languages list, `--lang` flag table, walker extensions, and Nerd Font icon table.
- `docs/reference/schema.json`: `"Go"` added to the `language` enum.
- `docs/rules/mcp.md`: `go`/`golang` added to the `lang` filter description for `scan`, `summary`, and `locate` tools.

## [0.4.6] — 2026-07-10

Go parser.

### Added

- **Go parser** (`src/lang/go.rs`): extracts `Function`, `Method`, `Struct`, and `Trait` (Go interface) entities from `.go` files. Handles `function_declaration`, `method_declaration`, `type_declaration` → `type_spec` → `struct_type`/`interface_type` (with `method_elem` children). Import detection via `import_declaration`, including grouped `import (...)` blocks, aliases, and paths. Relative imports (`./`, `../`) resolved as `Internal`; all other imports as `External`.

### Tests

- 15 new unit tests in `src/lang/go.rs`.
- 1 new integration test: `scan_go_file`.

## [0.4.5] — 2026-07-07

Language infrastructure refactor. No behavior changes; all existing test assertions unchanged.

### Changed

- **Generic parse pipeline** (`lang/mod.rs`): per-language `parse_all` functions removed. `LangDef` now carries `grammar`, `import_kinds`, `parse_node`, and `resolve_imports`; a single `run_parser` implements parse → entities → imports → errors for every language. Registering a language is now a single `LANGUAGES` entry plus one parser file.
- **`resolve_imports` dispatch** moved from a manual per-language `match` into the `LangDef` field.
- **Entity construction helpers** (`name_field`, `leaf_entity`, `container_entity`, `body_children`): replace ~20 repeated "name field → node lines → LOC count → Entity" blocks across all parsers.
- **JS/TS unified**: `js::parse_common` holds the shared node handling (functions, classes, methods, arrow functions, `export` wrappers) with a `recurse` callback; `ts.rs` keeps only TS-specific cases (interface, enum, namespace, abstract class, method signatures). TypeScript's `LangDef` reuses `js::resolve_imports` directly.
- **C/C++ unified** (`lang/c_family.rs`): shared `function_entity` (Function/Method by context), `unwrap_declaration`, and `resolve_includes` (previously byte-identical in `c.rs` and `cpp.rs`).
- **Shared path classifier** (`classify_path`): relative-prefix → `Internal`, otherwise `External`; used by JS/TS and Python.
- **`ParseOutput` struct**: replaces the `(Vec<Entity>, Vec<String>, Vec<SyntaxError>)` tuple.
- **`Context` enum** (`TopLevel`/`TypeBody`): replaces the opaque `in_class: bool` threaded through the walk.

### Tests

- 1 new unit test: `every_language_runs_through_the_generic_driver`.
- Parser unit tests now exercise the full pipeline via `test_parse`; C/C++/TS import-resolution tests now go through `lang::resolve_imports`, covering the `LangDef` wiring.

### Fixed

- **MCP: stale `lang` filter description** — the `scan`/`summary`/`locate` tool schemas listed `c, cpp, c++, js, py, qml, rust`, omitting the JS/Python aliases and TypeScript entirely. Now generated at runtime from the language registry (`lang_filter_description()`), so new languages can no longer drift.

### Documentation

- `docs/rules/parsers.md`: new Parse Pipeline section; Shared Helpers and Adding a Language rewritten for the `LangDef`-driven structure.
- `docs/rules/mcp.md`: Claude Code setup corrected — MCP servers are registered via `claude mcp add` / `.mcp.json`, not `.claude/settings.json`; added scope examples and `/mcp` verification step. `lang` filter tables updated with all names/aliases. Stale `0.2.7` in the scan response example replaced with `<code-seek-version>`.
- Reference docs caught up from 0.4.1 to 0.4.5: `SPECS.md` (status, language list incl. JS/TS/Python columns in the entity model table, `--lang`/`--info` flags, walker extensions, icons, future section), `ARCHITECTURE.md` (`scan/` module split, `LangDef` pipeline, parser module list), `DEPENDENCIES.md` (JS/Python/TS grammar crates, dev-dependencies note), `schema.json` (language enum).
- `CHECKLIST.md`: added Cycle 0.4.x table (0.4.2–0.4.5).

## [0.4.4] — 2026-07-07

TypeScript parser.

### Added

- **TypeScript parser** (`src/lang/ts.rs`): extracts entities from `.ts`, `.mts`, `.cts` files. Handles `function_declaration`, `class_declaration` and `abstract_class_declaration` (with methods, including `abstract_method_signature`), `interface_declaration` (mapped to `Trait`, with `method_signature` children), `enum_declaration`, `internal_module` (`namespace X {}`, mapped to `Namespace`, with nested entities), typed `const`/`let` arrow function assignments, and `export` wrappers. Type aliases are skipped (no matching entity type). ES module import detection (`import_statement`, including `import type`). Import resolution reuses the JavaScript resolver (same ES module semantics): relative imports as `Internal`, package imports as `External`. `.tsx` is not covered (requires the separate TSX grammar).

### Tests

- 15 new unit tests in `src/lang/ts.rs`.
- 1 new integration test: `scan_ts_file`.
- 1 new `detect_typescript` test in `src/lang/mod.rs`; `detect_unknown` now uses `.tsx` as its unknown-extension case.

## [0.4.3] — 2026-07-03

Python parser.

### Added

- **Python parser** (`src/lang/python.rs`): extracts `Function`, `Class`, and `Method` entities from `.py` and `.pyw` files. Handles `function_definition`, `async_function_definition`, `class_definition` (with methods), and `decorated_definition` wrappers. Import detection via `import_statement` and `import_from_statement`. Relative imports (`from .module`) resolved as `Internal`; absolute imports as `External`. Multi-name `import a, b, c` and `as` aliases supported.

### Tests

- 10 new unit tests in `src/lang/python.rs`.
- 1 new integration test: `scan_python_file`.
- 1 new `detect_python` test in `src/lang/mod.rs`.

## [0.4.2] — 2026-07-03

JavaScript parser.

### Added

- **JavaScript parser** (`src/lang/js.rs`): extracts `Function`, `Class`, and `Method` entities from `.js`, `.mjs`, `.cjs` files. Handles `function_declaration`, `class_declaration` (with methods), `const`/`let` arrow function assignments, and `export` wrappers. ES module import detection (`import_statement`). Relative imports resolved as `Internal`; package imports as `External`.

### Fixed

- **MCP docs: stale version** — session example now shows `<code-seek-version>` instead of hardcoded `0.2.7`.
- **MCP docs: `ignore` param undocumented** — added to the `scan` tool reference table.

### Documentation

- `docs/rules/mcp.md`: added PATH verification steps, manual server test instructions, and a troubleshooting section.
- `README.md`: added Rust/rustup install instructions and `~/.cargo/bin` PATH guidance.

### Tests

- 10 new unit tests in `src/lang/js.rs`.
- 1 new integration test: `scan_js_file`.
- 1 new `detect_javascript` test in `src/lang/mod.rs`.

## [0.4.1] — 2026-07-03

Security hardening & critical bug fixes.

### Fixed

- **`verify_internal_deps` broken** (critical): removed `verify_internal_deps` call from `resolve_dependencies` — `resolve_imports` already classifies deps correctly. Also removed the dead `verify_internal_deps` function from `lang/mod.rs`.
- **QML `import "path" as X` parsing** (important): fixed to parse content between first and second quote via `split('"').nth(1)` instead of broken `trim_matches('"')`.
- **Symlink cycle detection** (important): `walk_dir` now tracks visited directories via `HashSet<PathBuf>` of canonicalized paths to prevent stack overflow from recursive symlinks.
- **Absolute path guard in MCP** (important): reviewed — MCP clients always pass absolute paths, so blocking them breaks legitimate usage. Only `..` traversal is blocked. Proper fix (workspace root validation via MCP `initialize` `roots`) deferred to post-0.5.x.
- **Cache size cap** (medium): cache load now checks `fs::metadata` before reading; files above 100 MB are rejected with a warning.
- **`entities.len() - 1` fragile** (low): changed to `i + 1 == entities.len()` to avoid panic on empty entity slices.
- **Silent import drops** (low): `extract_imports_from_tree` now emits `log::warn` when `utf8_text()` fails on an import node.

### Added

- **`--info` filter** (CLI + MCP): new `--info` parameter for `code-seek scan` and MCP `scan` tool with values `all` (default), `no-tests` (excludes test modules), `tests-only` (shows only test modules).
- **Config default robustness**: `max_file_size_mb` in `[scan]` now correctly defaults to `10.0` even when the key is missing from a present `[scan]` section (previously silently defaulted to `0.0`, causing all files to be skipped).
- **`save_history_by_default` removed**: the `ScanConfig` field added in `0.2.1` was silently removed — the SQLite history feature it gated is deferred to post-0.6.x. The field is no longer parsed from `.code-seek/config.toml` (unknown fields are silently ignored by the TOML parser).

### Tests

- New QML test: `resolves_imports_with_alias` — verifies `import "path" as X` parsing.
- New cache test: `cache_load_oversized_file_returns_empty` — verifies 100 MB cap.
- 4 new `filter_by_info` unit tests covering `all`, `no-tests`, `tests-only`.
- 3 new integration tests for `--info` flag.

## [0.4.0] — 2026-07-02

Release — closes 0.3.x cycle (Dependencies & robustness). First public
release candidate. All planned features for 0.3.x are complete: entity
tree with JSON output, multi-language parsing (C, C++, Rust, QML), import
detection, dependency graph, syntax error detection, SHA-256 cache,
MCP server with 4 tools (scan, entity, summary, locate), `--ignore` flag,
language validation, and CLI filters (`--lang`, `--match`, `--max-depth`).

## [0.3.99] — 2026-07-02

### Added
- `--ignore <dirs>` CLI flag: comma-separated directory names to skip during walk, combined with `ignore_dirs` from config (e.g. `code-seek scan . --ignore target,node_modules`)
- MCP `scan` tool: optional `ignore` parameter (comma-separated directory names)
- 9 unit tests covering core modules: `cache` (load corrupt/missing/empty), `model` (`SyntaxError` fields, `FileResult` fields), `lang/detect` (path without extension, empty extension), `walker` (empty directory, case-sensitive ignore)
- 5 integration tests: `scan_with_ignore_flag_skips_dirs`, `scan_empty_rs_file`, `scan_only_comments_rs`, `scan_invalid_lang_cli`, `scan_symlink_root_no_follow`

### Changed
- `lang/mod.rs::parse_source`: replaced `.expect()` with `.ok()?` — eliminates the last potential panic in production code paths
- Visibility narrowed to `pub(super)`: `print_tree`, `print_json`, `json_entities`, `entity_path_segment`, `entity_type_str`, `count_entities` (in `render.rs`); `filter_entities` (in `filter.rs`)
- `build_scan_results` now validates the `--lang` filter and returns a clear error for unrecognized languages (e.g. `--lang python`)

## [0.3.3] — 2026-07-02

### Added
- Syntax error detection: `errors` field in JSON output per file — list of `{kind, node_kind, start_line, end_line}` objects; `kind` is `"error"` (ERROR node) or `"missing"` (MISSING node)
- `total_errors` top-level field in JSON output — sum across all files
- `lang::detect_syntax_errors(tree)`: shared helper that walks the tree-sitter AST for ERROR/MISSING nodes; does not recurse into children of ERROR nodes to avoid duplicate reporting
- Tree output shows error count in file header when errors are present (`1 error`, `2 errors`)
- Tree output shows `✓` or `⚠` after `[start-end]` on every entity line — `⚠` when any syntax error overlaps the entity's line range, `✓` otherwise
- Cache stores and restores `errors` per file (unlike `dependencies`, errors are not recomputed post-cache)
- 2 unit tests: `detect_error_node` (Rust invalid expression), `detect_missing_node` (C missing semicolon)
- 1 integration test: `scan_json_includes_errors_field` — verifies `errors` array, all fields, `total_errors`, and tree output

### Changed
- `ParseAll` type alias changed from pair to triple: `fn(&str, &LocMap) -> (Vec<Entity>, Vec<String>, Vec<SyntaxError>)`
- All four parsers (`c`, `cpp`, `rust`, `qml`) updated to return the triple

## [0.3.2] — 2026-07-02

### Added
- Dependency graph: JSON output per file includes `dependencies` array with `name` + `kind` (`internal`/`external`) — `#include <>` (C/C++), `use extern crate`/`use crate` (Rust), `import <Module>`/`import "./path"` (QML)
- Cross-reference resolution: quoted includes (`#include "file.h"`) and ambiguous Rust imports (`use serde::Serialize`) are checked against actual project files to determine internal vs external
- Cache stores resolved dependencies; internal deps are re-verified on cache load against current project file set
- Tree output shows dependency counts per file (`2 ext  1 int deps`)
- 4 unit tests for per-language import resolution (1 per language)
- 1 integration test: `scan_json_includes_dependencies_field` — verifies `dependencies` in both JSON and tree output

### Changed
- `model::FileResult` now includes `dependencies: Vec<Dependency>`
- `model::DependencyKind` (enum: `Internal`, `External`) and `model::Dependency` (name + kind)
- `cache::CachedEntry` stores `dependencies`; `cache::get`/`insert` updated
- `lang/mod.rs`: added `resolve_imports(language, imports, project_files)` dispatch and `verify_internal_deps(deps, project_files)` post-processing
- `scan/mod.rs::build_scan_results`: resolves dependencies against `HashSet<PathBuf>` of all scanned files
- `scan/render.rs`: JSON includes `dependencies` per file; tree output appends dep counts to file header

## [0.3.1] — 2026-07-02

### Added
- Import detection: each file in JSON output now includes an `imports` array with `#include` (C/C++), `use`/`extern crate` (Rust), and `import` (QML) declarations
- `lang::extract_imports_from_tree(tree, source, kinds)`: shared helper that walks a pre-built tree-sitter AST for import-like nodes — no parser created
- `lang::{c,cpp,rust,qml}::parse_all(source, loc_map) -> (Vec<Entity>, Vec<String>)`: each parser builds the AST once and returns entities and imports together, eliminating the previous double-parse per file
- `scan::scan_one`: per-file processing extracted from `build_scan_results` (detect, filter, size check, read, cache lookup, parse)
- `scan::apply_filters`: entity filter step extracted from `build_scan_results`
- `lang/loc.rs`: `LocMap` extracted to its own module
- 9 unit tests for import extraction (2 per language + Rust extern crate)
- 1 integration test: `scan_json_includes_imports_field` verifies `imports` field in JSON output
- Cache now stores and restores `imports` per file

### Changed
- `src/scan.rs` split into `src/scan/mod.rs` (orchestrator), `src/scan/render.rs` (tree/JSON output), `src/scan/filter.rs` (entity filtering, locate, summarize)
- `FileResult` now includes `imports: Vec<String>`
- `LangDef.parse_all` replaces the former `parser` + `imports` function pointer pair
- `model::FileResult` updated with `imports` field
- `CachedEntry` updated with `imports` field
- `parse_source`, `collect_children`, `node_lines`, `declarator_name`, `extract_imports_from_tree` narrowed from `pub(crate)` to `pub(super)` — internal to the `lang` module

## [0.3.0] — 2026-07-02

### Changed
- Removed `LanguageParser` trait and per-language parser structs (`CParser`, `CppParser`, `QmlParser`, `RustParser`); replaced with `parse_impl` free functions per language
- Construct `LocMap` once in `parse_file` and pass to all parsers (was built independently per-parser)
- Added depth limit (max 64) to `collect_children` to prevent stack overflow on deeply nested code
- Reorganized docs into subdirectories: `rules/`, `design/`, `reference/`, `temp/`
- Moved SPECS.md → `docs/reference/`, MCP.md → `docs/rules/`
- Created `docs/CHANGELOG.md`, `docs/design/`, `docs/RELEASE-PROCESS.md`

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
