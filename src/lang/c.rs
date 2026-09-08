use tree_sitter::Node;

use super::{Context, LocMap, c_family, leaf_entity, name_field};
use crate::model::{Entity, EntityType};

pub(super) fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" => c_family::function_entity(node, source, loc_map, context),
        "struct_specifier" => Some(leaf_entity(
            node,
            name_field(node, src)?,
            EntityType::Struct,
            loc_map,
        )),
        "enum_specifier" => Some(leaf_entity(
            node,
            name_field(node, src)?,
            EntityType::Enum,
            loc_map,
        )),
        "declaration" | "type_definition" => c_family::unwrap_declaration(
            node,
            source,
            loc_map,
            context,
            depth,
            &["struct_specifier", "enum_specifier"],
            parse_node,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DependencyKind, EntityType};

    fn parse(src: &str) -> Vec<Entity> {
        crate::lang::test_parse("C", src).entities
    }
    fn imports(src: &str) -> Vec<String> {
        crate::lang::test_parse("C", src).imports
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
        let deps = crate::lang::resolve_imports("C", &imports, &project);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "stdio.h");
        assert_eq!(deps[0].kind, DependencyKind::External);
        assert_eq!(deps[1].name, "util.h");
        assert_eq!(deps[1].kind, DependencyKind::Internal);
    }
}
