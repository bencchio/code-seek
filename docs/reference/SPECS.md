# Code Seek — Specification

## Overview

Code Seek is a CLI tool that explores the functional structure of a source code
repository. It produces a tree output with classes, functions, methods, and their
LOC, and stores historical snapshots for tracking code evolution.

## Status

**Current release: v0.2.7** (2026-07-01)

Implemented:
- `Cargo.toml` scaffold (edition 2024, clap 4)
- `code-seek init [--force]`: creates `.code-seek/config.toml` with `ignore_dirs` defaults
- File walker: recursive, ignores `.git/`, `.code-seek/`, and `ignore_dirs` from config; returns only supported extensions; respects `follow_symlinks` (default: false) and `max_file_size_mb` (default: 10)
- `code-seek scan <path> [--lang LANG] [--format FORMAT] [--match PATTERN] [--max-depth N]`: multi-language parsing (C, C++, Rust, QML), entity tree with LOC and line numbers, Nerd Font icons, `--lang` filter, `--format json` for structured output, `--match`/`--max-depth` entity filters
- Entity model: 8 types (Class, Enum, Function, Impl, Method, Namespace, Struct, Trait)
- Entity paths: canonical unique identifier (`file > [parent >]* name`) included in JSON output; impl/trait/mod containers use prefixed segments
- LOC counting: pre-computed prefix-sum per file (O(n) build, O(1) per entity); skips blank lines and comments (single-line and block)
- Error reporting: oversized files and unreadable files emit warnings to stderr and are skipped; scan never panics
- `code-seek mcp`: MCP server over stdio with 4 tools — `scan`, `entity`, `summary`, `locate`
- SHA256 incremental cache (`.code-seek/cache.json`): unchanged files are not re-parsed

## Technology Stack

- **Rust**: implementation language.
- **Tree-sitter**: multi-language parsing (never crashes, detects syntax errors).
- **TOML**: configuration format (`.code-seek/config.toml`).
- **SHA-256**: incremental cache to skip re-parsing unchanged files.
- **JSON-RPC 2.0**: MCP protocol for AI agent consumption.
- **Single binary**: no Python, Node.js, or Java runtime dependencies.

## Abstract Entity Model

Every detected entity maps to a canonical type, regardless of language:

| EntityType  | Description                      | C/C++     | Rust   | QML      |
|-------------|----------------------------------|-----------|--------|----------|
| `Function`  | Top-level function               | function  | fn     | —        |
| `Method`    | Method inside class/impl/trait   | method    | fn     | function |
| `Class`     | Class or QML object              | class     | —      | object   |
| `Struct`    | Structure                        | struct    | struct | —        |
| `Trait`     | Trait / interface                | —         | trait  | —        |
| `Impl`      | Impl block                       | —         | impl   | —        |
| `Enum`      | Enumeration                      | enum      | enum   | —        |
| `Namespace` | Module / namespace               | namespace | mod    | —        |

### Entity attributes

Implemented (v0.2.0):

- `name`: short name
- `entity_type`: one of EntityType
- `loc`: effective lines of code (no blanks, no comments)
- `start_line`, `end_line`: absolute lines in the file
- `children`: nested entities (methods inside impl/trait/class)

Implemented (v0.2.2):

- `entity_path`: unique canonical string (`path > [parent >]* name`); included in JSON output

Planned (v0.3+):

- `kind`: language-specific variant string (e.g. `"struct"`, `"trait"`)
- `signature`: full declaration text (optional)

### Anonymous entities

Lambda functions, closures, and arrow functions without a name receive a
generated identifier: `<anonymous>_1`, `<anonymous>_2`, etc., numbered per file.
(v0.3+)

## Entity Path

Canonical format that uniquely identifies any entity:

```
<relative-path> > [<parent-entity> >]* <name>
```

The separator is `>`. Whitespace around it is optional during parsing.

Examples:

```
src/main.cpp > Parser > parse
src/main.cpp > main
src/user.rs > User
src/user.rs > impl User > create
src/user.rs > impl User > login
src/user.rs > bootstrap
src/types.rs > Status
src/types.rs > Status > Active
src/main.qml > MainWindow
src/main.qml > MainWindow > onButtonClick
```

## Supported Languages

### v0.2.3 (current)

- C
- C++
- Rust
- QML

### Post-0.4.x (planned)

- Python
- JavaScript
- TypeScript

## CLI Commands

### v0.2.7 (current)

#### `code-seek init`

Initialize a Code Seek project in the current directory.
Creates `.code-seek/` with default `config.toml`.

Flags:

- `--force`: overwrite existing files

#### `code-seek mcp`

Start a JSON-RPC 2.0 MCP server over stdio.

Exposes 4 tools:
- `scan` — entity index of a directory/file with filters
- `entity` — single entity lookup by `entity_path`
- `summary` — aggregate counts (files, LOC, entities by language/type)
- `locate` — case-insensitive entity name search across all files

See [`docs/rules/mcp.md`](../rules/mcp.md) for full reference.

#### `code-seek scan <path>`

Analyze a file or directory and print the structural tree or JSON.

Flags:

- `--lang <langs>`: filter by language; comma-separated for multiple (`--lang rust,cpp`). Supported values: `c`, `cpp`, `c++`, `qml`, `rust`.
- `--format <format>`: output format; `tree` (default) or `json`.
- `--match <pattern>`: case-insensitive substring filter on entity name. Parents are preserved when a child matches. Files with zero matching entities still show their header.
- `--max-depth <n>`: hard depth ceiling. `0` = file headers only, `1` = root entities, `2` = root + direct children. No flag = unlimited.

Flags compose: `--max-depth` is applied first (hard ceiling), then `--match` filters within the resulting tree.

### v0.4.x (planned)

#### `code-seek config get [key]`
#### `code-seek config set <key> <value>`
#### `code-seek config list`

#### `code-seek ignore add <pattern>`
#### `code-seek ignore remove <pattern>`
#### `code-seek ignore list`

#### `code-seek scan --format ndjson`
#### `code-seek scan --filter "loc > N"`
#### `code-seek diff --from HEAD~1`

## Output Format

### Tree Format

```
󱘗 src/config.rs  [Rust]                              19 LOC  3 entities
├─ 󰠱 Config                                           3 LOC  [  6-  8]
├─ 󰠱 ScanConfig                                       3 LOC  [ 12- 14]
└─ 󰊕 load                                             7 LOC  [ 16- 22]

󱘗 src/main.rs  [Rust]                                38 LOC  3 entities
├─ 󰠱 Cli                                              4 LOC  [ 14- 17]
├─ 󰒻 Command                                         11 LOC  [ 20- 30]
└─ 󰊕 main                                            11 LOC  [ 32- 42]

2 files  6 entities
```

Line format:

```
<language-icon> <file-path>  [<language>]  <LOC> LOC  <entity-count> entity/entities
<tree-connector> <entity-icon> <name>  <LOC> LOC  [<start-line>-<end-line>]
```

Rules:

- Files sorted alphabetically by path.
- Entities sorted alphabetically within their parent.
- Tree connectors: `├─` for middle siblings, `└─` for last child, `│` for continuation.
- Language icons: Nerd Font characters (󰙱 C, 󰙲 C++, 󱘗 Rust, 󰈚 QML).
- Entity icons: 󰊕 Function/Method, 󰌗 Class, 󰠱 Struct, 󰒻 Enum, 󰅩 Namespace, 󰜁 Trait, 󰉺 Impl.
- Columns are globally aligned (label width, LOC width, line-number width).
- Effective LOC: blank lines and comments are ignored.

Summary at the end:

```
2 files  6 entities
```

### JSON Format

Selected with `--format json`. Produces a single pretty-printed JSON object.

Top-level schema:

```json
{
  "version": "<current>",
  "scan_path": "<path>",
  "total_files": 2,
  "total_loc": 66,
  "total_entities": 7,
  "files": [ ... ]
}
```

Per-file entry:

```json
{
  "path": "src/model.rs",
  "language": "Rust",
  "loc": 28,
  "entity_count": 3,
  "entities": [ ... ]
}
```

Per-entity entry (recursive via `children`):

```json
{
  "entity_path": "src/model.rs > impl Entity > new",
  "name": "new",
  "entity_type": "Method",
  "loc": 3,
  "start_line": 25,
  "end_line": 27,
  "children": []
}
```

Entity path segment rules:

- `Impl` → `"impl <Name>"`
- `Trait` → `"trait <Name>"`
- `Namespace` → `"mod <Name>"`
- All others → `"<Name>"`

Examples:

```
src/foo.rs > impl Foo > bar
src/foo.rs > trait Animal > speak
src/foo.rs > mod utils > helper
src/foo.rs > Point
```

### Walker

The walker always ignores `.git/` and `.code-seek/` directories.
Respects `ignore_dirs`, `follow_symlinks`, and `max_file_size_mb` from `.code-seek/config.toml`.
Returns only files with supported extensions (`.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`, `.rs`, `.qml`).
Supports both directory and single-file paths.
Unreadable directories and oversized files emit a warning to stderr and are skipped.

## Configuration System

File: `.code-seek/config.toml` — created by `code-seek init`.

Fields implemented in v0.2.0:

```toml
version = "0.1"

[scan]
follow_symlinks = false      # skip symlinks during walk
max_file_size_mb = 10        # skip files larger than this
ignore_dirs = ["target", "node_modules", ...]
```

CLI commands for reading/writing config (`config get/set/list`) are planned for v0.4.2.

## Ignore System (v0.4.3)

File: `.code-seek/checkignore`

Syntax compatible with `.gitignore`.

Default example:

```
.git/
target/
node_modules/
dist/
build/
__pycache__/
.venv/
*.min.js
generated/*
```

## History System (post-0.4.x)

### SQLite Schema

File: `.code-seek/history.db`

```sql
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL,
    code-seek_version TEXT NOT NULL,
    total_files INTEGER DEFAULT 0,
    total_loc INTEGER DEFAULT 0,
    total_entities INTEGER DEFAULT 0,
    duration_ms INTEGER DEFAULT 0
);

CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    snapshot_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    language TEXT NOT NULL,
    loc INTEGER DEFAULT 0,
    sha256 TEXT,
    has_errors INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    UNIQUE(snapshot_id, path),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id)
);

CREATE TABLE entities (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL,
    snapshot_id INTEGER NOT NULL,
    entity_path TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    kind TEXT,
    parent_entity_path TEXT,
    loc INTEGER DEFAULT 0,
    start_line INTEGER,
    end_line INTEGER,
    signature TEXT,
    FOREIGN KEY (file_id) REFERENCES files(id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id)
);

CREATE INDEX idx_entities_snapshot ON entities(snapshot_id);
CREATE INDEX idx_entities_path ON entities(entity_path);
```

### Diffs

```sql
-- Entities added in B vs A
SELECT entity_path FROM entities WHERE snapshot_id = 2
EXCEPT
SELECT entity_path FROM entities WHERE snapshot_id = 1;

-- Entities removed in B vs A
SELECT entity_path FROM entities WHERE snapshot_id = 1
EXCEPT
SELECT entity_path FROM entities WHERE snapshot_id = 2;

-- Entities modified
SELECT a.entity_path,
       a.loc AS old_loc, b.loc AS new_loc
FROM entities a
JOIN entities b ON a.entity_path = b.entity_path
WHERE a.snapshot_id = 1 AND b.snapshot_id = 2
  AND (a.loc != b.loc
       OR a.start_line != b.start_line
       OR a.end_line != b.end_line);
```

## Incremental Cache (v0.2.6)

Each file is hashed with SHA-256 during scan and stored in
`.code-seek/cache.json` alongside parsed entities. If the same file is scanned
again with an identical hash, entities are loaded from cache instead of
re-parsing.

Cache behavior:
- Cache stored at `.code-seek/cache.json` as compact JSON
- Keyed by relative file path; entry stores `sha256`, `language`, `loc`, `entities`
- On miss or hash mismatch, the file is re-parsed and cache is updated
- Cache is written only when at least one file was re-parsed (`cache_dirty` flag)
- Missing or corrupt cache file is silently ignored (fresh start)
- Missing `.code-seek/` directory skips cache write silently — no error

## Error Handling

Code Seek never panics on problematic input.

Other handled errors:

- Binary unparseable files: skipped with warning.
- Files exceeding `max_file_size_mb`: skipped with warning.
- Non-UTF-8 files: skipped with warning.
- Unreadable directories: skipped with warning.

## Future

### v0.3.x — Dependencies & robustness

- Import/include/use/require detection (C, C++, Rust, QML)
- Dependency graph in JSON output (internal vs external)
- Syntax error detection (ERROR/MISSING nodes in output)
- MCP progress tokens (`$/progress`) and client cancellation signal
- Stabilization (v0.3.99) / Release (v0.4.0)

### v0.4.x — Configuration & integration

- Config system (`config get/set/list`)
- Ignore system (`checkignore` patterns)
- `--format ndjson` streaming output
- `--filter "loc > N"` attribute expressions
- Git-aware diff (`code-seek diff --from HEAD~1`)

### Post-0.4.x

- Unify C/C++ parsers (factor duplicated `parse_node` logic)
- Python, JavaScript, TypeScript parsers
- SQLite history (`scan --save`, `history list/show/diff`)
- Markdown export
- Structure visualizer (standalone, consumes JSON)
- TUI (Terminal User Interface)
- Duplicate code, overly long function, and circular dependency detection
- Historical metrics and trends
- Plugin system for new languages

## Architecture Notes

- **Deterministic ordering**: files and entities are sorted alphabetically to
  prevent false positives in diffs.
- **Per-language Tree-sitter parser**: each language has a separate module
  implementing `parse(source: &str) -> Vec<Entity>`.
- **SHA-256 cache**: incremental scanning avoids re-parsing unchanged files
  (`.code-seek/cache.json`).
