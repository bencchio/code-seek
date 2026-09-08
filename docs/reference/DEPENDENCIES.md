# Dependencies

## Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4 | CLI argument parsing (derive macro) |
| `serde` | 1 | Serialization/deserialization (cache, entity model) |
| `serde_json` | 1 | JSON output, MCP message format |
| `sha2` | 0.10 | SHA-256 hashing for incremental cache |
| `toml` | 1 | Configuration file parsing (`config.toml` in the XDG slot) |
| `tree-sitter` | 0.26.9 | Incremental parsing engine |
| `tree-sitter-c` | 0.24 | C grammar (functions, structs, enums) |
| `tree-sitter-cpp` | 0.23 | C++ grammar (classes, namespaces, methods) |
| `tree-sitter-elixir` | 0.3 | Elixir grammar (defmodule, def, defp, defmacro) |
| `tree-sitter-javascript` | 0.25 | JavaScript grammar (functions, classes, methods, arrow functions) |
| `tree-sitter-python` | 0.25 | Python grammar (functions, classes, methods, decorators) |
| `tree-sitter-qmljs` | 0.3.1 | QML grammar (object definitions, functions) |
| `tree-sitter-go` | 0.25 | Go grammar (functions, methods, structs, interfaces) |
| `tree-sitter-rust` | 0.24 | Rust grammar (fn, struct, impl, trait, enum, mod) |
| `tree-sitter-typescript` | 0.23 | TypeScript grammar (JS constructs + interfaces, enums, namespaces); TSX grammar unused |

## Notes

- `tree-sitter-language` (transitive) — bridges the `LanguageFn` type used by parser crates ≥0.23 with the tree-sitter 0.26 `Language` type.
- All parser crates expose a `LANGUAGE` constant of type `LanguageFn` (`tree-sitter-typescript` exposes `LANGUAGE_TYPESCRIPT`/`LANGUAGE_TSX`); language registration uses `.into()` to convert to `tree_sitter::Language`.
- Grammar crates version independently of the `tree-sitter` crate; they are at latest crates.io stable.
- Dev-dependencies: `tree-sitter` 0.26.9 (same crate as runtime; used directly in `lang/mod.rs` tests).
