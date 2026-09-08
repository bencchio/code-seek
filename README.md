# Code Seek

Explore the functional structure of a repository from the CLI.

- **Structural tree**: classes, methods, functions, structs, traits, enums,
  components, with LOC and line numbers.
- **Multi-language**: C, C++, Rust, QML.
- **Error-tolerant**: partially parses even with invalid code.

## Status

**v0.4.0** — First public release candidate. Closes the 0.3.x cycle
(Dependencies & robustness). All planned features complete: entity tree,
JSON output, multi-language parsing, import/dependency/error detection,
SHA-256 cache, MCP server, and CLI filters.

**v0.3.4** — Stabilization. `--ignore` CLI flag and MCP `ignore` parameter to skip
directories at runtime. Dead code removed. Last production panic eliminated.

See [`docs/rules/mcp.md`](docs/rules/mcp.md) for MCP setup.

## Installation

**Requirements:** Rust toolchain (stable).

```bash
# Build release binary and install system-wide
cargo build --release
sudo cp target/release/code-seek /usr/local/bin/code-seek
```

Or install into `~/.cargo/bin` (must be on `PATH`):

```bash
cargo install --path .
```

Verify:

```bash
code-seek --version
```

## Quick start

```bash
# Initialize Code Seek in the current directory
code-seek init

# Analyze the codebase
code-seek scan .
```

## Using with AI agents

There are two ways to give an AI agent access to code-seek: instruction files (teach the agent to run the CLI) and MCP (give the agent a structured tool).

### Instruction files — CLAUDE.md / AGENTS.md

Add this to your project's `CLAUDE.md` (Claude Code) or `AGENTS.md` (Codex / OpenAI):

```markdown
## Code structure

Use `code-seek scan <path>` to explore the structure of any file or directory before reading source files.

- Overview of the whole repo: `code-seek scan .`
- JSON output for structured data: `code-seek scan . --format json`
- Filter by language: `code-seek scan . --lang rust`
- Find an entity by name: `code-seek scan . --match parse`
- Limit tree depth: `code-seek scan . --max-depth 1`

Prefer `--max-depth 1` for a fast first pass on large directories.
```

This works with any agent that can run shell commands. No MCP setup required.

### MCP — programmatic tool access

`code-seek mcp` starts a JSON-RPC 2.0 server over stdio with 4 tools:
`scan`, `entity`, `summary`, `locate`.

See [`docs/rules/mcp.md`](docs/rules/mcp.md) for setup instructions (Claude Code, OpenCode) and tool reference.

## Sample output

```
󱘗 src/main.rs  [Rust]          38 LOC  3 entities
├─ 󰠱 Cli                        4 LOC  [ 14- 17]
├─ 󰒻 Command                   11 LOC  [ 20- 30]
└─ 󰊕 main                      11 LOC  [ 32- 42]

󱘗 src/model.rs  [Rust]         28 LOC  3 entities
├─ 󰠱 Entity                     8 LOC  [ 16- 23]
├─ 󰒻 EntityType                10 LOC  [  4- 13]
└─ 󰠱 FileResult                 6 LOC  [ 26- 31]

2 files  6 entities
```

## Documentation

- [`docs/reference/SPECS.md`](docs/reference/SPECS.md) — Full specification

## Supported languages

C · C++ · Rust · QML
