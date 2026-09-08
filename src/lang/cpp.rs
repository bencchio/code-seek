use tree_sitter::Node;

use super::{LocMap, collect_children, declarator_name, node_lines, parse_source};
use crate::model::{Entity, EntityType};

pub(super) fn parse_impl(source: &str, loc_map: &LocMap) -> Vec<Entity> {
    let Some(tree) = parse_source(tree_sitter_cpp::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    collect_children(tree.root_node(), source, loc_map, false, 0, parse_node)
}

fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    in_class: bool,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" => {
            let name = declarator_name(node.child_by_field_name("declarator")?, src)?;
            let (start_line, end_line) = node_lines(node);
            let entity_type = if in_class { EntityType::Method } else { EntityType::Function };
            Some(Entity::new(
                name,
                entity_type,
                loc_map.count(start_line, end_line),
                start_line,
                end_line,
            ))
        }
        "class_specifier" | "struct_specifier" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let entity_type = if node.kind() == "class_specifier" {
                EntityType::Class
            } else {
                EntityType::Struct
            };
            let (start_line, end_line) = node_lines(node);
            let children = node
                .child_by_field_name("body")
                .map(|body| {
                    collect_children(body, source, loc_map, true, depth + 1, parse_node)
                })
                .unwrap_or_default();
            Some(Entity {
                name,
                entity_type,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children,
            })
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
        "namespace_definition" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)
                .unwrap_or_else(|| "<anonymous>".to_string());
            let (start_line, end_line) = node_lines(node);
            let children = node
                .child_by_field_name("body")
                .map(|body| {
                    collect_children(body, source, loc_map, false, depth + 1, parse_node)
                })
                .unwrap_or_default();
            Some(Entity {
                name,
                entity_type: EntityType::Namespace,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children,
            })
        }
        "declaration" | "type_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(
                    child.kind(),
                    "struct_specifier" | "enum_specifier" | "class_specifier"
                ) {
                    return parse_node(child, source, loc_map, in_class, depth);
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
    fn parses_class_with_methods() {
        let src = "class Vec {\npublic:\n    float dot() { return 0; }\n    float length() { return 1; }\n};";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Vec");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 2);
        assert_eq!(entities[0].children[0].name, "dot");
        assert_eq!(entities[0].children[1].name, "length");
        assert!(
            entities[0]
                .children
                .iter()
                .all(|c| c.entity_type == EntityType::Method)
        );
    }

    #[test]
    fn parses_namespace_with_function() {
        let src = "namespace math { float clamp(float v) { return v; } }";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "math");
        assert_eq!(entities[0].entity_type, EntityType::Namespace);
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].name, "clamp");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_struct_with_method() {
        let src = "struct Pair { int first() { return a; } int a; };";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Pair");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].entity_type, EntityType::Method);
    }

    #[test]
    fn parses_top_level_function() {
        let entities = parse("int square(int x) { return x * x; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "square");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_enum() {
        let entities = parse("enum class Color { Red, Green, Blue };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn empty_source() {
        assert_eq!(parse("").len(), 0);
    }
}
