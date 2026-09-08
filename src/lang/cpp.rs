use tree_sitter::Node;

use super::{Context, LocMap, body_children, c_family, container_entity, leaf_entity, name_field};
use crate::model::{Entity, EntityType};

pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context, depth: usize) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" => c_family::function_entity(node, source, loc_map, context),
        "class_specifier" | "struct_specifier" => {
            let name = name_field(node, src)?;
            let entity_type = if node.kind() == "class_specifier" {
                EntityType::Class
            } else {
                EntityType::Struct
            };
            let children = body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(node, name, entity_type, loc_map, children))
        }
        "enum_specifier" => Some(leaf_entity(node, name_field(node, src)?, EntityType::Enum, loc_map)),
        "namespace_definition" => {
            let name = name_field(node, src).unwrap_or_else(|| "<anonymous>".to_string());
            let children = body_children(node, source, loc_map, Context::TopLevel, depth, parse_node);
            Some(container_entity(node, name, EntityType::Namespace, loc_map, children))
        }
        "declaration" | "type_definition" => c_family::unwrap_declaration(
            node,
            source,
            loc_map,
            context,
            depth,
            &["struct_specifier", "enum_specifier", "class_specifier"],
            parse_node,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DependencyKind, EntityType};

    fn parse(src: &str) -> Vec<Entity> { crate::lang::test_parse("C++", src).entities }
    fn imports(src: &str) -> Vec<String> { crate::lang::test_parse("C++", src).imports }

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

    #[test]
    fn extracts_includes() {
        let src = "#include <vector>\n#include \"util.hpp\"\nint main() {}";
        let result = imports(src);
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("vector"));
        assert!(result[1].contains("util.hpp"));
    }

    #[test]
    fn resolves_includes() {
        let imports = vec!["#include <vector>".into(), "#include \"util.hpp\"".into()];
        let project = std::collections::HashSet::from([std::path::PathBuf::from("util.hpp")]);
        let deps = crate::lang::resolve_imports("C++", &imports, &project);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "vector");
        assert_eq!(deps[0].kind, DependencyKind::External);
        assert_eq!(deps[1].name, "util.hpp");
        assert_eq!(deps[1].kind, DependencyKind::Internal);
    }
}
