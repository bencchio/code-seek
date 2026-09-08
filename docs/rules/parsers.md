# Parser Rules

## Parser Goals

All parsers should:

- Emit the same entity model (EntityType, Entity)
- Follow the same naming conventions
- Preserve deterministic ordering (alphabetical within parent)
- Expose structural information — never evaluate code quality

## LOC Counting

LOC counting uses `LocMap`, a prefix-sum structure that pre-computes effective LOC per line once per file:

- O(n) build, O(1) per entity lookup
- Blank lines and comments are excluded (single-line and block comments)
- `LocMap::count(start, end)` returns total effective LOC between two lines

## Shared Helpers

All parsers use these from `lang/mod.rs`:

- `parse_source(language, source)` — initializes a tree-sitter parser with `.expect()` and returns the parsed tree
- `collect_children(node, source, loc_map, context, depth, f)` — generic iterator that walks tree-sitter children; guards against infinite recursion at `depth >= 64`; callback `f` signature: `Fn(Node, &str, &LocMap, bool, usize) -> Option<Entity>`
- `Entity::new(name, entity_type, loc, start_line, end_line)` — constructor for leaf entities (children always empty)

## Entity Path Rules

Entity paths are constructed by `entity_path_segment()`:

| EntityType | Segment format     | Example                        |
|------------|--------------------|--------------------------------|
| Function   | `<name>`           | `main`                         |
| Method     | `<name>`           | `new`                          |
| Class      | `<name>`           | `Config`                       |
| Struct     | `<name>`           | `Point`                        |
| Enum       | `<name>`           | `Status`                       |
| Impl       | `impl <name>`      | `impl Foo`                     |
| Trait      | `trait <name>`     | `trait Animal`                 |
| Namespace  | `mod <name>`       | `mod utils`                    |

Full path: `<file_path> > [parent_segment >]* segment`

## Adding a Language

1. **Add dependency** — add tree-sitter grammar crate to `Cargo.toml`
2. **Implement parser** — create `src/lang/<lang>.rs` with:
   ```rust
   pub(super) fn parse_impl(source: &str, loc_map: &LocMap) -> Vec<Entity> { ... }
   ```
   `LocMap` is provided by the caller — do not build it inside `parse_impl`
3. **Register language** — in `lang/mod.rs`:
   - Add a private wrapper: `fn parse_<lang>(src: &str, loc_map: &LocMap) -> Vec<Entity> { <lang>::parse_impl(src, loc_map) }`
   - Add a `LangDef` entry to `LANGUAGES` (canonical name, extensions, aliases, icon, `parser: parse_<lang>`)
4. **Add tests** — unit tests in the parser module calling `parse_impl` directly; integration tests in `tests/`

## Consistency Rules

- All `pub` items must be `pub(crate)` — this is a binary crate with no external consumers
- `EntityType` derives: `Debug`, `PartialEq`, `Clone`, `Serialize`, `Deserialize`
- `Entity` derives: `Debug`, `Clone`, `Serialize`, `Deserialize`
- Avoid language-specific output structures when a common representation exists
- Test naming: `describe_behavior()` for unit tests, integration tests in `tests/` directory
