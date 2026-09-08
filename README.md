# Codexa

Explore the functional structure of a repository from the CLI.

- **Structural tree**: classes, methods, functions, structs, traits, enums,
  components, with LOC and line numbers.
- **Multi-language**: C, C++, Rust, QML (v0.1.x). Python, JS/TS (v0.2.x).
- **History**: snapshots with SQLite to track changes and run diffs.
- **Incremental**: SHA256 hashing avoids re-parsing unchanged files.
- **Error-tolerant**: partially parses even with invalid code.

## Status

MVP with CLI scaffold, `init` and `scan` commands, and file walker.

## Quick start

```bash
# Initialize Codexa in the current directory
codexa init

# Analyze the codebase (v0.1.x)
codexa scan .
```

From v0.2.x:

```bash
# Save snapshot and view history
codexa scan . --save
codexa history list
codexa history show <snapshot-id>
codexa history diff <a> <b>

# Configure
codexa config set scan.follow_symlinks true
codexa config list

# Ignore directories
codexa ignore add "target/"
codexa ignore list
```

## Sample output (v0.1.x)

```
src/main.cpp  (120 LOC, C++)
  Classes:
    Parser
      parse()      12 LOC  [5-16]
      validate()    8 LOC  [18-25]
  Functions:
    main()         15 LOC  [28-42]

src/user.rs  (45 LOC, Rust)
  Impl User:
    create()       14 LOC  [3-16]
    login()         7 LOC  [18-24]
  Functions:
    bootstrap()    18 LOC  [27-44]
```

## Documentation

- [`docs/SPECS.md`](docs/SPECS.md) — Full specification

## Supported languages

### v0.1.x
C · C++ · Rust · QML

### v0.2.x
Python · JavaScript · TypeScript
