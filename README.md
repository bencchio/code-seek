# Code Seek

Explore the functional structure of a repository from the CLI.

- **Structural tree**: classes, methods, functions, structs, traits, enums,
  components, with LOC and line numbers.
- **Multi-language**: C, C++, Rust, QML (v0.1.x). Python, JS/TS (v0.2.x).
- **History**: snapshots with SQLite to track changes and run diffs.
- **Incremental**: SHA256 hashing avoids re-parsing unchanged files.
- **Error-tolerant**: partially parses even with invalid code.

## Status

**v0.2.0** — stable release. `code-seek scan` parses C, C++, Rust, and QML; counts
effective LOC; renders a tree with 8 entity types (Class, Enum, Function, Impl, Method,
Namespace, Struct, Trait); supports `--lang` filtering; enforces `follow_symlinks`
(default: false) and `max_file_size_mb` (default: 10) from `.code-seek/config.toml`.

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

# Analyze the codebase (v0.1.x)
code-seek scan .
```

From v0.2.x:

```bash
# Save snapshot and view history
code-seek scan . --save
code-seek history list
code-seek history show <snapshot-id>
code-seek history diff <a> <b>

# Configure
code-seek config set scan.follow_symlinks true
code-seek config list

# Ignore directories
code-seek ignore add "target/"
code-seek ignore list
```

## Sample output (v0.1.x)

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

- [`docs/SPECS.md`](docs/SPECS.md) — Full specification

## Supported languages

### v0.1.x
C · C++ · Rust · QML

### v0.2.x
Python · JavaScript · TypeScript
