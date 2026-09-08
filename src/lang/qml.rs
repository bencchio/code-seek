use tree_sitter::Node;

use super::{Context, LocMap, container_entity, leaf_entity, name_field};
use crate::model::{Dependency, DependencyKind, Entity, EntityType};

pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, _context: Context, _depth: usize) -> Option<Entity> {
    if node.kind() == "ui_object_definition" {
        parse_object(node, source, loc_map)
    } else {
        None
    }
}

fn parse_object(node: Node<'_>, source: &str, loc_map: &LocMap) -> Option<Entity> {
    let src = source.as_bytes();
    let type_name = node
        .child_by_field_name("type_name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_owned)?;

    let initializer = node.child_by_field_name("initializer")?;
    let mut id_name: Option<String> = None;
    let mut methods: Vec<Entity> = Vec::new();

    let mut cursor = initializer.walk();
    for child in initializer.children(&mut cursor) {
        match child.kind() {
            "ui_binding" if id_name.is_none() => {
                id_name = extract_id(&child, src);
            }
            "function_declaration" => {
                if let Some(m) = parse_function(child, source, loc_map) {
                    methods.push(m);
                }
            }
            _ => {}
        }
    }

    methods.sort_by(|a, b| a.name.cmp(&b.name));

    let name = id_name.unwrap_or(type_name);
    Some(container_entity(node, name, EntityType::Class, loc_map, methods))
}

fn extract_id(binding: &Node<'_>, src: &[u8]) -> Option<String> {
    let name_node = binding.child_by_field_name("name")?;
    if name_node.utf8_text(src).ok()? != "id" {
        return None;
    }
    let value = binding.child_by_field_name("value")?;
    value.named_child(0)?.utf8_text(src).ok().map(str::to_owned)
}

fn parse_function(node: Node<'_>, source: &str, loc_map: &LocMap) -> Option<Entity> {
    let name = name_field(node, source.as_bytes())?;
    Some(leaf_entity(node, name, EntityType::Method, loc_map))
}

pub(super) fn resolve_imports(
    imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim().strip_prefix("import ")?.trim();
            if let Some(quoted) = raw.split('"').nth(1) {
                let name = quoted.to_owned();
                return Some(Dependency { name, kind: DependencyKind::Internal });
            }
            let name = raw.split_whitespace().next()?.to_owned();
            Some(Dependency { name, kind: DependencyKind::External })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;

    fn entities(s: &str) -> Vec<Entity> { crate::lang::test_parse("QML", s).entities }
    fn imports(s: &str) -> Vec<String> { crate::lang::test_parse("QML", s).imports }

    #[test]
    fn parses_object_with_id() {
        let result = entities("import QtQuick 2.0\nRectangle { id: root }");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "root");
        assert_eq!(result[0].entity_type, EntityType::Class);
    }

    #[test]
    fn uses_type_name_when_no_id() {
        let result = entities("import QtQuick 2.0\nRectangle { width: 100 }");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Rectangle");
        assert_eq!(result[0].entity_type, EntityType::Class);
    }

    #[test]
    fn parses_functions_as_methods() {
        let src = "import QtQuick 2.0\nItem {\n    id: root\n    function reset() { }\n    function update(x) { }\n}";
        let result = entities(src);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "root");
        assert_eq!(result[0].children.len(), 2);
        assert!(result[0].children.iter().all(|c| c.entity_type == EntityType::Method));
    }

    #[test]
    fn sorts_methods_alphabetically() {
        let src = "import QtQuick 2.0\nItem { id: root\n    function zoo() {}\n    function alpha() {}\n}";
        let result = entities(src);
        let methods = &result[0].children;
        assert_eq!(methods[0].name, "alpha");
        assert_eq!(methods[1].name, "zoo");
    }

    #[test]
    fn skips_nested_objects() {
        let src = "import QtQuick 2.0\nRectangle { id: root\n    Button { id: btn }\n    function click() {}\n}";
        let result = entities(src);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[0].children[0].name, "click");
    }

    #[test]
    fn empty_source() {
        assert_eq!(entities("").len(), 0);
    }

    #[test]
    fn extracts_imports() {
        let src = "import QtQuick 2.15\nimport QtQuick.Controls 2.15\nItem {}";
        let result = imports(src);
        assert_eq!(result.len(), 2, "got {result:?}");
        assert!(result[0].contains("QtQuick"));
        assert!(result[1].contains("Controls"));
    }

    #[test]
    fn resolves_imports() {
        let imports = vec![
            "import QtQuick 2.15".into(),
            "import \"./components\"".into(),
        ];
        let project = std::collections::HashSet::new();
        let deps = resolve_imports(&imports, &project);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "QtQuick");
        assert_eq!(deps[0].kind, DependencyKind::External);
        assert_eq!(deps[1].name, "./components");
        assert_eq!(deps[1].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolves_imports_with_alias() {
        let imports = vec![
            "import \"path/to/local\" as MyModule".into(),
            "import QtQuick 2.15".into(),
        ];
        let project = std::collections::HashSet::new();
        let deps = resolve_imports(&imports, &project);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "path/to/local");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
        assert_eq!(deps[1].name, "QtQuick");
        assert_eq!(deps[1].kind, DependencyKind::External);
    }
}
