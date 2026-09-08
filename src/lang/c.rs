use std::path::Path;

use tree_sitter::Node;

use super::{LocMap, collect_children, declarator_name, detect_syntax_errors, extract_imports_from_tree, node_lines, parse_source};
use crate::model::{Dependency, DependencyKind, Entity, EntityType, SyntaxError};

pub(super) fn parse_all(source: &str, loc_map: &LocMap) -> (Vec<Entity>, Vec<String>, Vec<SyntaxError>) {
    let Some(tree) = parse_source(tree_sitter_c::LANGUAGE.into(), source) else {
        return (vec![], vec![], vec![]);
    };
    let entities = collect_children(tree.root_node(), source, loc_map, false, 0, |n, s, lm, _, _| {
        parse_node(n, s, lm)
    });
    let imports = extract_imports_from_tree(&tree, source, &["preproc_include"]);
    let errors = detect_syntax_errors(&tree);
    (entities, imports, errors)
}

fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" => {
            let name = declarator_name(node.child_by_field_name("declarator")?, src)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity::new(
                name,
                EntityType::Function,
                loc_map.count(start_line, end_line),
                start_line,
                end_line,
            ))
        }
        "struct_specifier" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity::new(
                name,
                EntityType::Struct,
                loc_map.count(start_line, end_line),
                start_line,
                end_line,
            ))
        }
        "enum_specifier" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity::new(
                name,
                EntityType::Enum,
                loc_map.count(start_line, end_line),
                start_line,
                end_line,
            ))
        }
        "declaration" | "type_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "struct_specifier" | "enum_specifier") {
                    return parse_node(child, source, loc_map);
                }
            }
            None
        }
        _ => None,
    }
}

pub(super) fn resolve_imports(
    imports: &[String],
    project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.starts_with("#include") {
                let inner = raw.trim_start_matches("#include").trim();
                let (content, is_angle) = if inner.starts_with('<') {
                    (inner.trim_matches(|c| c == '<' || c == '>'), true)
                } else if inner.starts_with('"') {
                    (inner.trim_matches('"'), false)
                } else {
                    return None;
                };
                let name = content.to_owned();
                let kind = if is_angle {
                    DependencyKind::External
                } else {
                    let file_path = Path::new(&name);
                    if project_files.iter().any(|p| p.ends_with(file_path)) {
                        DependencyKind::Internal
                    } else {
                        DependencyKind::External
                    }
                };
                Some(Dependency { name, kind })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;

    fn parse(src: &str) -> Vec<Entity> { parse_all(src, &LocMap::build(src)).0 }
    fn imports(src: &str) -> Vec<String> { parse_all(src, &LocMap::build(src)).1 }

    #[test]
    fn parses_function() {
        let entities = parse("int add(int a, int b) { return a + b; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
        assert_eq!(entities[0].start_line, 1);
        assert_eq!(entities[0].end_line, 1);
    }

    #[test]
    fn parses_named_struct() {
        let entities = parse("struct Point { int x; int y; };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn parses_named_enum() {
        let entities = parse("enum Color { RED, GREEN, BLUE };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn parses_typedef_named_struct() {
        let entities = parse("typedef struct Foo { int x; } FooAlias;");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn skips_anonymous_struct() {
        let entities = parse("struct { int x; };");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn empty_source() {
        assert_eq!(parse("").len(), 0);
    }

    #[test]
    fn multiple_entities_sorted() {
        let src = "void zoo() {} void alpha() {} void mid() {}";
        let entities = parse(src);
        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0].name, "alpha");
        assert_eq!(entities[1].name, "mid");
        assert_eq!(entities[2].name, "zoo");
    }

    #[test]
    fn extracts_includes() {
        let src = "#include <stdio.h>\n#include \"util.h\"\nint main() {}";
        let result = imports(src);
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("stdio.h"));
        assert!(result[1].contains("util.h"));
    }

    #[test]
    fn resolves_includes() {
        let imports = vec!["#include <stdio.h>".into(), "#include \"util.h\"".into()];
        let project = std::collections::HashSet::from([std::path::PathBuf::from("util.h")]);
        let deps = resolve_imports(&imports, &project);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "stdio.h");
        assert_eq!(deps[0].kind, DependencyKind::External);
        assert_eq!(deps[1].name, "util.h");
        assert_eq!(deps[1].kind, DependencyKind::Internal);
    }
}
