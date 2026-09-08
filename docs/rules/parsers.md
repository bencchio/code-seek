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

All parsers use these from `lang/mod.rs` (all `pub(super)` — internal to the `lang` module):

- `parse_source(language, source)` — initializes a tree-sitter parser and returns the parsed `Tree`
- `extract_imports_from_tree(tree, source, kinds)` — walks root children of an already-built tree and collects text of nodes matching `kinds`; no parser created
- `collect_children(node, source, loc_map, context, depth, f)` — generic iterator that walks tree-sitter children; guards against infinite recursion at `depth >= 64`; callback `f` signature: `Fn(Node, &str, &LocMap, bool, usize) -> Option<Entity>`
- `Entity::new(name, entity_type, loc, start_line, end_line)` — constructor for leaf entities (children always empty)

`LocMap` lives in `lang/loc.rs` and is re-exported from `lang/mod.rs`.

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
2. **Implement parser** — create `src/lang/<lang>.rs` with a single entry point:
   ```rust
   pub(super) fn parse_all(source: &str, loc_map: &LocMap) -> (Vec<Entity>, Vec<String>) {
       let Some(tree) = parse_source(LANG::LANGUAGE.into(), source) else {
           return (vec![], vec![]);
       };
       let entities = /* walk tree for entities */;
       let imports = extract_imports_from_tree(&tree, source, &["import_node_kind"]);
       (entities, imports)
   }
   ```
   Parse the AST once and extract both entities and imports from the same tree.
3. **Register language** — add a `LangDef` entry to `LANGUAGES` in `lang/mod.rs`:
   ```rust
   LangDef { canonical: "Lang", aliases: &["lang"], extensions: &["ext"], icon: "󰀀", parse_all: lang::parse_all }
   ```
4. **Add tests** — in the parser module:
   ```rust
   fn parse(src: &str) -> Vec<Entity> { parse_all(src, &LocMap::build(src)).0 }
   fn imports(src: &str) -> Vec<String> { parse_all(src, &LocMap::build(src)).1 }
   ```
   Integration tests in `tests/`

## Consistency Rules

- Public API (`detect`, `parse_file`, `LANGUAGES`, `LocMap`) must be `pub(crate)` — binary crate, no external consumers
- Parser helpers (`parse_source`, `extract_imports_from_tree`, `collect_children`, `node_lines`, `declarator_name`) must be `pub(super)` — internal to the `lang` module only
- `EntityType` derives: `Debug`, `PartialEq`, `Clone`, `Serialize`, `Deserialize`
- `Entity` derives: `Debug`, `Clone`, `Serialize`, `Deserialize`
- Avoid language-specific output structures when a common representation exists
- Test naming: `describe_behavior()` for unit tests, integration tests in `tests/` directory
