use tree_sitter::Node;

use super::{
    Context, LocMap, classify_path, collect_children, container_entity, leaf_entity, name_field,
};
use crate::model::{Dependency, Entity, EntityType};

pub(super) fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    _context: Context,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_declaration" => Some(leaf_entity(
            node,
            name_field(node, src)?,
            EntityType::Function,
            loc_map,
        )),
        "method_declaration" => Some(leaf_entity(
            node,
            name_field(node, src)?,
            EntityType::Method,
            loc_map,
        )),
        "type_declaration" => parse_type_declaration(node, source, loc_map, depth),
        _ => None,
    }
}

fn parse_type_declaration(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_spec" {
            let name = name_field(child, src)?;
            if let Some(entity) = parse_type_body(child, name, source, loc_map, depth + 1) {
                return Some(entity);
            }
        }
    }
    None
}

fn parse_type_body(
    node: Node<'_>,
    name: String,
    source: &str,
    loc_map: &LocMap,
    depth: usize,
) -> Option<Entity> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "struct_type" => {
                return Some(leaf_entity(node, name, EntityType::Struct, loc_map));
            }
            "interface_type" => {
                let children = collect_children(
                    child,
                    source,
                    loc_map,
                    Context::TypeBody,
                    depth + 1,
                    |n, s, l, _c, _d| {
                        let src = s.as_bytes();
                        match n.kind() {
                            "method_elem" => {
                                Some(leaf_entity(n, name_field(n, src)?, EntityType::Method, l))
                            }
                            _ => None,
                        }
                    },
                );
                return Some(container_entity(
                    node,
                    name,
                    EntityType::Trait,
                    loc_map,
                    children,
                ));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn resolve_imports(
    imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .flat_map(|raw| extract_paths(raw))
        .map(|path| classify_path(path, &["./", "../"]))
        .collect()
}

fn extract_paths(raw: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            let rest = trimmed.strip_prefix("import ").unwrap_or(trimmed).trim();
            if rest.starts_with('(') {
                continue;
            }
            if let Some(p) = extract_quoted(rest) {
                paths.push(p);
            }
        } else if trimmed.starts_with('"') || trimmed.starts_with('\'') {
            if let Some(p) = extract_quoted(trimmed) {
                paths.push(p);
            }
        } else if !trimmed.is_empty()
            && trimmed != ")"
            && let Some(p) = extract_quoted(trimmed)
        {
            paths.push(p);
        }
    }
    paths
}

fn extract_quoted(s: &str) -> Option<String> {
    let s = s.trim();
    for quote in ['"', '\''] {
        if let Some(start) = s.find(quote) {
            let after = &s[start + 1..];
            if let Some(end) = after.find(quote) {
                let path = &after[..end];
                if !path.is_empty() {
                    return Some(path.to_owned());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DependencyKind;

    fn parse(src: &str) -> Vec<Entity> {
        crate::lang::test_parse("Go", src).entities
    }
    fn imports(src: &str) -> Vec<String> {
        crate::lang::test_parse("Go", src).imports
    }

    #[test]
    fn collects_block_imports() {
        let src = "import (\n\t\"fmt\"\n\t\"os\"\n)\nfunc main() {}";
        let raw = imports(src);
        assert_eq!(raw.len(), 1);
        assert!(raw[0].contains("fmt"));
        assert!(raw[0].contains("os"));
    }

    #[test]
    fn collects_imports() {
        let src = "import \"fmt\"\nimport \"os\"\nfunc main() {}";
        let raw = imports(src);
        assert_eq!(raw.len(), 2);
        assert!(raw.iter().any(|i| i.contains("fmt")));
        assert!(raw.iter().any(|i| i.contains("os")));
    }

    #[test]
    fn detects_syntax_error() {
        let src = "func foo( { return }";
        let errors = crate::lang::test_parse("Go", src).errors;
        assert!(!errors.is_empty());
    }

    #[test]
    fn empty_source() {
        assert_eq!(parse("").len(), 0);
    }

    #[test]
    fn parses_function() {
        let entities = parse("func add(a, b int) int { return a + b }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_interface() {
        let src = "type Animal interface {\n    Speak() string\n    Move(dx, dy int)\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Animal");
        assert_eq!(entities[0].entity_type, EntityType::Trait);
        assert_eq!(entities[0].children.len(), 2);
        assert!(
            entities[0]
                .children
                .iter()
                .all(|c| c.entity_type == EntityType::Method)
        );
        assert!(entities[0].children.iter().any(|c| c.name == "Speak"));
        assert!(entities[0].children.iter().any(|c| c.name == "Move"));
    }

    #[test]
    fn parses_method() {
        let src = "type Foo struct {}\nfunc (f *Foo) Bar() int { return 0 }";
        let entities = parse(src);
        assert_eq!(entities.len(), 2);
        let method = entities
            .iter()
            .find(|e| e.entity_type == EntityType::Method)
            .unwrap();
        assert_eq!(method.name, "Bar");
    }

    #[test]
    fn parses_multiple_top_level() {
        let src = "func alpha() {}\nfunc beta() {}";
        let entities = parse(src);
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn parses_only_package() {
        let src = "package main";
        let entities = parse(src);
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn parses_struct() {
        let src = "type Point struct { X int; Y int }";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn resolve_imports_alias_stripped() {
        let raw = vec!["import alias \"github.com/foo/bar\"".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "github.com/foo/bar");
    }

    #[test]
    fn resolve_imports_block() {
        let raw = vec!["import (\n\t\"fmt\"\n\t\"os\"\n)".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn resolve_imports_relative_is_internal() {
        let raw = vec!["import \"./local/pkg\"".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "./local/pkg");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolve_imports_stdlib_is_external() {
        let raw = vec!["import \"fmt\"".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "fmt");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }

    #[test]
    fn resolve_imports_third_party_is_external() {
        let raw = vec!["import \"github.com/foo/bar\"".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "github.com/foo/bar");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }

    #[test]
    fn skips_type_alias() {
        let entities = parse("type X = string");
        assert!(entities.is_empty());
    }

    #[test]
    fn sorts_alphabetically() {
        let src = "func zoo() {}\nfunc alpha() {}\nfunc mid() {}";
        let entities = parse(src);
        assert_eq!(entities[0].name, "alpha");
        assert_eq!(entities[1].name, "mid");
        assert_eq!(entities[2].name, "zoo");
    }
}
