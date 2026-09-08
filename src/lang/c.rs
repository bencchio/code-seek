use tree_sitter::{Node, Parser};

use super::{LanguageParser, LocMap, declarator_name, node_lines};
use crate::model::{Entity, EntityType};

pub struct CParser;

impl LanguageParser for CParser {
    fn parse(&self, source: &str) -> Vec<Entity> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
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
        "struct_specifier" => {
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
        "declaration" | "type_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "struct_specifier" | "enum_specifier") {
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
    fn parses_function() {
        let entities = CParser.parse("int add(int a, int b) { return a + b; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
        assert_eq!(entities[0].start_line, 1);
        assert_eq!(entities[0].end_line, 1);
    }

    #[test]
    fn parses_named_struct() {
        let entities = CParser.parse("struct Point { int x; int y; };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn parses_named_enum() {
        let entities = CParser.parse("enum Color { RED, GREEN, BLUE };");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn parses_typedef_named_struct() {
        let entities = CParser.parse("typedef struct Foo { int x; } FooAlias;");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Struct);
    }

    #[test]
    fn skips_anonymous_struct() {
        let entities = CParser.parse("struct { int x; };");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    fn empty_source() {
        assert_eq!(CParser.parse("").len(), 0);
    }

    #[test]
    fn multiple_entities_sorted() {
        let src = "void zoo() {} void alpha() {} void mid() {}";
        let entities = CParser.parse(src);
        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0].name, "alpha");
        assert_eq!(entities[1].name, "mid");
        assert_eq!(entities[2].name, "zoo");
    }
}
