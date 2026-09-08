use tree_sitter::{Node, Parser};

use super::{LanguageParser, LocMap, node_lines};
use crate::model::{Entity, EntityType};

pub struct RustParser;

impl LanguageParser for RustParser {
    fn parse(&self, source: &str) -> Vec<Entity> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let Some(tree) = parser.parse(source.as_bytes(), None) else {
            return Vec::new();
        };
        let loc_map = LocMap::build(source);
        parse_nodes(tree.root_node(), source, &loc_map, false)
    }
}

fn parse_nodes(node: Node<'_>, source: &str, loc_map: &LocMap, in_impl: bool) -> Vec<Entity> {
    let mut entities = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(e) = parse_node(child, source, loc_map, in_impl) {
            entities.push(e);
        }
    }
    entities.sort_by(|a, b| a.name.cmp(&b.name));
    entities
}

fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, in_impl: bool) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_item" | "function_signature_item" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity {
                name,
                entity_type: if in_impl {
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
        "struct_item" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            Some(Entity {
                name,
                entity_type: EntityType::Struct,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children: Vec::new(),
            })
        }
        "enum_item" => {
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
        "impl_item" => {
            let name = impl_name(node, src)?;
            let (start_line, end_line) = node_lines(node);
            let children = node
                .child_by_field_name("body")
                .map(|body| parse_nodes(body, source, loc_map, true))
                .unwrap_or_default();
            Some(Entity {
                name,
                entity_type: EntityType::Impl,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children,
            })
        }
        "trait_item" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            let children = node
                .child_by_field_name("body")
                .map(|body| parse_nodes(body, source, loc_map, true))
                .unwrap_or_default();
            Some(Entity {
                name,
                entity_type: EntityType::Trait,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children,
            })
        }
        "mod_item" => {
            let body = node.child_by_field_name("body")?;
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            let children = parse_nodes(body, source, loc_map, false);
            Some(Entity {
                name,
                entity_type: EntityType::Namespace,
                loc: loc_map.count(start_line, end_line),
                start_line,
                end_line,
                children,
            })
        }
        _ => None,
    }
}

fn impl_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    extract_type_name(node.child_by_field_name("type")?, src)
}

fn extract_type_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" => node.utf8_text(src).ok().map(str::to_owned),
        "generic_type" => node
            .child_by_field_name("type")
            .and_then(|n| extract_type_name(n, src)),
        "scoped_type_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
            .map(str::to_owned),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::LanguageParser;
    use crate::model::EntityType;

    #[test]
    fn parses_function() {
        let entities = RustParser.parse("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_struct() {
        let entities = RustParser.parse("struct Point { x: f32, y: f32 }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn parses_enum() {
        let entities = RustParser.parse("enum Color { Red, Green, Blue }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn parses_impl_with_methods() {
        let src = "impl Foo {\n    fn bar(&self) {}\n    fn baz(&self) {}\n}";
        let entities = RustParser.parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
        assert_eq!(entities[0].children.len(), 2);
        assert_eq!(entities[0].children[0].name, "bar");
        assert_eq!(entities[0].children[1].name, "baz");
        assert!(
            entities[0]
                .children
                .iter()
                .all(|c| c.entity_type == EntityType::Method)
        );
    }

    #[test]
    fn parses_generic_impl() {
        let src = "impl<T> Container<T> {\n    fn len(&self) -> usize { 0 }\n}";
        let entities = RustParser.parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Container");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
    }

    #[test]
    fn parses_trait_impl() {
        let src =
            "impl Display for Foo {\n    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }\n}";
        let entities = RustParser.parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
    }

    #[test]
    fn parses_trait_with_methods() {
        let src = "trait Animal {\n    fn name(&self) -> &str;\n    fn speak(&self) {}\n}";
        let entities = RustParser.parse(src);
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
    }

    #[test]
    fn parses_mod_with_items() {
        let src = "mod utils {\n    fn helper() {}\n}";
        let entities = RustParser.parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "utils");
        assert_eq!(entities[0].entity_type, EntityType::Namespace);
        assert_eq!(entities[0].children[0].name, "helper");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Function);
    }

    #[test]
    fn skips_mod_without_body() {
        let entities = RustParser.parse("mod foo;");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn empty_source() {
        assert_eq!(RustParser.parse("").len(), 0);
    }

    #[test]
    fn sorts_alphabetically() {
        let src = "fn zoo() {} fn alpha() {} fn mid() {}";
        let entities = RustParser.parse(src);
        assert_eq!(entities[0].name, "alpha");
        assert_eq!(entities[1].name, "mid");
        assert_eq!(entities[2].name, "zoo");
    }
}
