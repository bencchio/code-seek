use tree_sitter::Node;

use super::{LocMap, collect_children, node_lines, parse_source};
use crate::model::{Entity, EntityType};

pub(super) fn parse_impl(source: &str, loc_map: &LocMap) -> Vec<Entity> {
    let Some(tree) = parse_source(tree_sitter_rust::LANGUAGE.into(), source) else {
        return Vec::new();
    };
    collect_children(tree.root_node(), source, loc_map, false, 0, parse_node)
}

fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    in_impl: bool,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_item" | "function_signature_item" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned)?;
            let (start_line, end_line) = node_lines(node);
            let entity_type = if in_impl { EntityType::Method } else { EntityType::Function };
            Some(Entity::new(
                name,
                entity_type,
                loc_map.count(start_line, end_line),
                start_line,
                end_line,
            ))
        }
        "struct_item" => {
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
        "enum_item" => {
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
        "impl_item" => {
            let name = impl_target_type(node, src)?;
            let (start_line, end_line) = node_lines(node);
            let children = node
                .child_by_field_name("body")
                .map(|body| {
                    collect_children(body, source, loc_map, true, depth + 1, parse_node)
                })
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
                .map(|body| {
                    collect_children(body, source, loc_map, true, depth + 1, parse_node)
                })
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
            let children =
                collect_children(body, source, loc_map, false, depth + 1, parse_node);
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

fn impl_target_type(node: Node<'_>, src: &[u8]) -> Option<String> {
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
    use crate::model::EntityType;

    fn parse(src: &str) -> Vec<Entity> {
        parse_impl(src, &LocMap::build(src))
    }

    #[test]
    fn parses_function() {
        let entities = parse("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_struct() {
        let entities = parse("struct Point { x: f32, y: f32 }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn parses_enum() {
        let entities = parse("enum Color { Red, Green, Blue }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn parses_impl_with_methods() {
        let src = "impl Foo {\n    fn bar(&self) {}\n    fn baz(&self) {}\n}";
        let entities = parse(src);
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
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Container");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
    }

    #[test]
    fn parses_trait_impl() {
        let src =
            "impl Display for Foo {\n    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
    }

    #[test]
    fn parses_trait_with_methods() {
        let src = "trait Animal {\n    fn name(&self) -> &str;\n    fn speak(&self) {}\n}";
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
    }

    #[test]
    fn parses_mod_with_items() {
        let src = "mod utils {\n    fn helper() {}\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "utils");
        assert_eq!(entities[0].entity_type, EntityType::Namespace);
        assert_eq!(entities[0].children[0].name, "helper");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Function);
    }

    #[test]
    fn skips_mod_without_body() {
        let entities = parse("mod foo;");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn empty_source() {
        assert_eq!(parse("").len(), 0);
    }

    #[test]
    fn sorts_alphabetically() {
        let src = "fn zoo() {} fn alpha() {} fn mid() {}";
        let entities = parse(src);
        assert_eq!(entities[0].name, "alpha");
        assert_eq!(entities[1].name, "mid");
        assert_eq!(entities[2].name, "zoo");
    }
}
