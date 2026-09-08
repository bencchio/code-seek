use tree_sitter::Node;

use super::{LocMap, collect_children, declarator_name, node_lines, parse_source};
use crate::model::{Entity, EntityType};

pub(super) fn parse_impl(source: &str, loc_map: &LocMap) -> Vec<Entity> {
    let Some(tree) = parse_source(tree_sitter_c::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    collect_children(tree.root_node(), source, loc_map, false, 0, |n, s, lm, _, _| {
        parse_node(n, s, lm)
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;

    fn parse(src: &str) -> Vec<Entity> {
        parse_impl(src, &LocMap::build(src))
    }

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
}
