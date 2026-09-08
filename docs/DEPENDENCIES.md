# Dependencies

## Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4 | CLI argument parsing (derive macro) |
| `tree-sitter` | 0.25 | Incremental parsing engine |
| `tree-sitter-c` | 0.24 | C grammar (functions, structs, enums) |
| `tree-sitter-cpp` | 0.23 | C++ grammar (classes, namespaces, methods) |
| `tree-sitter-qmljs` | 0.3 | QML grammar (object definitions, functions) |
| `tree-sitter-rust` | 0.24 | Rust grammar (fn, struct, impl, trait, enum, mod) |

## Notes

- `tree-sitter-language` (transitive) — bridges the `LanguageFn` type used by parser crates ≥0.23 with the tree-sitter 0.25 `Language` type.
- All parser crates expose a `LANGUAGE` constant of type `LanguageFn`; language registration uses `.into()` to convert to `tree_sitter::Language`.
- No dev-only or build-time dependencies beyond what Cargo manages automatically.
