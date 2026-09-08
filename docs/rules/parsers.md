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

## Parse Pipeline

Parsing is driven by a single generic pipeline in `lang/mod.rs` (`run_parser`):
parse the source once, then extract entities, imports, and syntax errors from
the same tree. Each `LangDef` entry supplies the language-specific data:

- `grammar` — tree-sitter grammar constructor
- `import_kinds` — root-level node kinds collected as raw imports
- `parse_node` — callback that maps one node to an `Entity` (or `None`)
- `resolve_imports` — classifies raw imports into `Internal`/`External` dependencies

Parser files contain only `parse_node`, its language-specific helpers, and
(where not shared) `resolve_imports`.

## Shared Helpers

All parsers use these from `lang/mod.rs` (all `pub(super)` — internal to the `lang` module):

- `Context` — enum (`TopLevel` | `TypeBody`) threaded through the walk; parsers use it to decide Function vs Method and to reject members outside a container
- `collect_children(node, source, loc_map, context, depth, f)` — generic iterator that walks tree-sitter children, sorts results alphabetically; guards against infinite recursion at `depth >= 64`; callback `f` signature: `Fn(Node, &str, &LocMap, Context, usize) -> Option<Entity>` (`ParseNode` is the fn-pointer alias)
- `name_field(node, src)` — text of the node's `name` field, the common naming shape across grammars
- `leaf_entity(node, name, entity_type, loc_map)` — entity without children; computes lines and LOC from the node
- `container_entity(node, name, entity_type, loc_map, children)` — entity with children
- `body_children(node, source, loc_map, context, depth, f)` — entities of the node's `body` field; empty when the node has no body
- `classify_path(name, internal_prefixes)` — path-based import classification (relative → `Internal`, package → `External`); used by JS/TS/Python
- `declarator_name(node, src)` — unwraps C-family declarators to the identifier
- `parse_source(language, source)` — initializes a tree-sitter parser and returns the parsed `Tree`
- `extract_imports_from_tree(tree, source, kinds)` — walks root children of an already-built tree and collects text of nodes matching `kinds`
- `test_parse(canonical, source)` — `#[cfg(test)]` helper: runs the full pipeline for a registered language; parser unit tests build their `parse`/`imports` helpers on it

Language-family bases:

- `lang/c_family.rs` — shared by C and C++: `function_entity` (Function/Method by context), `unwrap_declaration` (`typedef`/`declaration` wrappers), `resolve_includes` (`#include` angle/quote classification)
- `js::parse_common` — shared by JavaScript and TypeScript: functions, classes, methods, arrow-function bindings, `export` wrappers; takes the language's `parse_node` as `recurse` so wrappers re-enter language-specific cases. TypeScript also reuses `js::resolve_imports` directly

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
2. **Implement parser** — create `src/lang/<lang>.rs` with a `parse_node`
   callback (and `resolve_imports` if no existing one fits):
   ```rust
   pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context, depth: usize) -> Option<Entity> {
       let src = source.as_bytes();
       match node.kind() {
           "function_node_kind" => Some(leaf_entity(node, name_field(node, src)?, EntityType::Function, loc_map)),
           "class_node_kind" => {
               let name = name_field(node, src)?;
               let children = body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
               Some(container_entity(node, name, EntityType::Class, loc_map, children))
           }
           _ => None,
       }
   }
   ```
3. **Register language** — add a `LangDef` entry to `LANGUAGES` in `lang/mod.rs`
   (the single point of change; the generic pipeline does the rest):
   ```rust
   LangDef {
       canonical: "Lang",
       aliases: &["lang"],
       extensions: &["ext"],
       icon: "󰀀",
       grammar: || tree_sitter_lang::LANGUAGE.into(),
       import_kinds: &["import_node_kind"],
       parse_node: lang::parse_node,
       resolve_imports: lang::resolve_imports,
   }
   ```
4. **Add tests** — in the parser module:
   ```rust
   fn parse(src: &str) -> Vec<Entity> { crate::lang::test_parse("Lang", src).entities }
   fn imports(src: &str) -> Vec<String> { crate::lang::test_parse("Lang", src).imports }
   ```
   Integration tests in `tests/`

## Consistency Rules

- Public API (`detect`, `parse_file`, `resolve_imports`, `LANGUAGES`, `LocMap`) must be `pub(crate)` — binary crate, no external consumers
- Parser helpers (everything listed under Shared Helpers, plus `node_lines`) must be `pub(super)` — internal to the `lang` module only; `LangDef`'s `grammar`, `import_kinds`, `parse_node`, and `resolve_imports` fields stay private to `lang`
- `EntityType` derives: `Debug`, `PartialEq`, `Clone`, `Serialize`, `Deserialize`
- `Entity` derives: `Debug`, `Clone`, `Serialize`, `Deserialize`
- Avoid language-specific output structures when a common representation exists
- Test naming: `describe_behavior()` for unit tests, integration tests in `tests/` directory
