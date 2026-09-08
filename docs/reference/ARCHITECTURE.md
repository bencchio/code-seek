# Architecture

## How This Repo Works

Code Seek extracts structural information from source code and presents it in a deterministic format for both humans and AI agents.

## High-Level Flow

```
Input Source Code
  → Walker (file discovery)
  → SHA-256 Cache (skip unchanged files)
  → Language Detection (extension-based)
  → Tree-sitter Parsing (per-language)
  → Entity Extraction (normalized model)
  → Filtering (match / max-depth)
  → Structured Output (tree or JSON)
  → CLI and MCP Consumers
```

## Module Responsibilities

### `src/main.rs`
CLI entry point using clap. Defines three subcommands: `init`, `scan`, `mcp`. Parses CLI arguments and dispatches to the corresponding module.

### `src/model.rs`
Shared entity model used across all parsers and outputs:
- `EntityType` — 8 canonical types: Class, Enum, Function, Impl, Method, Namespace, Struct, Trait
- `Entity` — name, type, loc, line range, children (recursive)
- `FileResult` — path, language, loc, entity list per file

### `src/scan/` (`mod.rs`, `render.rs`, `filter.rs`)
Orchestrator for `code-seek scan`. `mod.rs` calls the walker to discover files, loads the SHA-256 cache, delegates parsing to `lang::parse_file`, and exposes `build_scan_results`, `results_to_json`, `find_entity`, `summarize`, and `locate` for MCP consumption. `filter.rs` applies `--match`/`--max-depth`/`--info` filters; `render.rs` renders tree output with aligned columns.

### `src/lang/mod.rs`
Language registry (`LANGUAGES` static array) and the generic parse pipeline. Each `LangDef` entry carries the language's grammar, root-level import node kinds, `parse_node` callback, and `resolve_imports` function; `run_parser()` implements parse → entities → imports → errors once for all languages. Shared helpers:
- `detect()` — language detection by file extension
- `parse_file()` / `resolve_imports()` — public API dispatching through the registry
- `parse_source()` — centralized tree-sitter parser initialization
- `collect_children()` / `body_children()` — generic node walkers (alphabetical order, depth guard)
- `name_field()`, `leaf_entity()`, `container_entity()` — entity construction
- `classify_path()` — relative-vs-package import classification
- `Context` (`TopLevel`/`TypeBody`), `ParseOutput` — walk context and pipeline result types

### `src/lang/c.rs`, `cpp.rs`, `js.rs`, `python.rs`, `qml.rs`, `rust.rs`, `ts.rs`
Per-language parsers. Each implements `parse_node(node, source, loc_map, context, depth) -> Option<Entity>` mapping tree-sitter nodes to the canonical entity model — no language-specific output structures. Family bases hold shared handling: `c_family.rs` for C/C++ (`#include` resolution, function/declaration handling) and `js::parse_common` for JS/TS.

### `src/mcp.rs`
JSON-RPC 2.0 MCP server over stdio. Implements `initialize`, `tools/list`, and `tools/call` with 4 tools: `scan`, `entity`, `summary`, `locate`. Negotiates protocol version with the client. Logs all requests to stderr via `crate::log`.

### `src/walker.rs`
Recursive file walker. Respects `ignore_dirs`, `follow_symlinks`, and `max_file_size_mb` from config. Always ignores `.git/` and `.code-seek/`. Returns only supported file extensions.

### `src/config.rs`
Loads `.code-seek/config.toml`. Provides defaults for all fields when file is absent.

### `src/init.rs`
Implements `code-seek init` — creates `.code-seek/` directory with default `config.toml` and pre-filled `ignore_dirs`.

### `src/cache.rs`
SHA-256 incremental scan cache. Stores per-file hash, language, LOC, and entities in `.code-seek/cache.json`. On subsequent scans, files with matching hashes are loaded from cache instead of re-parsed.

### `src/log.rs`
Structured stderr logging with `info`, `warn`, and `error` functions. Output is prefixed with `[INFO]`, `[WARN]`, or `[ERROR]`.

## Architectural Goals

- Deterministic output
- Shared internal model
- Consistent entity representation
- Agent-friendly consumption
- Caching for incremental performance
- Error tolerance (never panic on malformed input)

## Key Design Decisions

- MCP support is mandatory — every feature added to the CLI should also be accessible via MCP
- Deterministic output is mandatory — alphabetical sorting prevents false positives in diffs
- Cache is optional and silent — missing `.code-seek/` directory never produces errors
- `pub(crate)` visibility — binary crate with no external API surface
- Per-language modules share helpers via `lang/mod.rs` — adding a language requires one registry entry and one parser module
