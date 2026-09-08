use tree_sitter::{Node, Parser};

use super::{LanguageParser, LocMap, declarator_name, node_lines};
use crate::model::{Entity, EntityType};

pub struct CppParser;

impl LanguageParser for CppParser {
    fn parse(&self, source: &str) -> Vec<Entity> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .unwrap();
        let Some(tree) = parser.parse(source.as_bytes(), None) else {
            return Vec::new();
        };
        let loc_map = LocMap::build(source);
        parse_nodes(tree.root_node(), source, &loc_map, false)
    }
}

fn parse_nodes(node: Node<'_>, source: &str, loc_map: &LocMap, in_class: bool) -> Vec<Entity> {
    let mut entities = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(e) = parse_node(child, source, loc_map, in_class) {
            entities.push(e);
        }
    }
    entities.sort_by(|a, b| a.name.cmp(&b.name));
    entities
}

fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, in_class: bool) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" => {
            let name = declarator_name(node.child_by_field_name("declarator")?, src)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity {
                name,
                entity_type: if in_class {
                    EntityType::Method
                } else {
                    EntityType::Function
                },
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children: Vec::new(),
            })
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
                .map(|body| parse_nodes(body, source, loc_map, true))
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
            Some(Entity {
                name,
                entity_type: EntityType::Enum,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children: Vec::new(),
            })
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
                .map(|body| parse_nodes(body, source, loc_map, false))
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
                    return parse_node(child, source, loc_map, in_class);
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
    use crate::lang::LanguageParser;
    use crate::model::EntityType;

    #[test]
    fn parses_class_with_methods() {
        let src = "class Vec {\npublic:\n    float dot() { return 0; }\n    float length() { return 1; }\n};";
        let entities = CppParser.parse(src);
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
        let entities = CppParser.parse(src);
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
        let entities = CppParser.parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Pair");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].entity_type, EntityType::Method);
    }

    #[test]
    fn parses_top_level_function() {
        let entities = CppParser.parse("int square(int x) { return x * x; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "square");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_enum() {
        let entities = CppParser.parse("enum class Color { Red, Green, Blue };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn empty_source() {
        assert_eq!(CppParser.parse("").len(), 0);
    }
}
