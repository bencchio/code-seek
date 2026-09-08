# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

## [0.2.0] — 2026-06-30

Release — closes the 0.1.x MVP cycle.

### Changed
- `EntityType`: removed unused `Clone` and `Eq` derives — only `Debug` and `PartialEq` remain
- `Entity`: removed unused `Clone` derive — only `Debug` remains
- `LanguageParser`: narrowed visibility from `pub` to `pub(crate)` (binary crate, no external consumers)
- `parse_file`: replaced unreachable `_ => Vec::new()` arm with `unreachable!()` to communicate the invariant

## [0.1.7] — 2026-06-30

### Changed
- Cleared completed 0.1 work from the backlog ahead of the release.

## [0.1.6] — 2026-06-30

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
