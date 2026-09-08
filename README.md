# Code Seek

Explore the functional structure of a repository from the CLI.

- **Structural tree**: classes, methods, functions, structs, traits, enums,
  components, with LOC and line numbers.
- **Multi-language**: C, C++, Rust, QML.
- **Error-tolerant**: partially parses even with invalid code.

## Status

MCP server and scan CLI for C, C++, Rust, and QML, with JSON, match, and depth filters.
See [`docs/rules/mcp.md`](docs/rules/mcp.md) for setup.

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
