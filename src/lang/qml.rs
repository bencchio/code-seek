use tree_sitter::Node;

use super::{LocMap, detect_syntax_errors, extract_imports_from_tree, node_lines, parse_source};
use crate::model::{Dependency, DependencyKind, Entity, EntityType, SyntaxError};

pub(super) fn parse_all(source: &str, loc_map: &LocMap) -> (Vec<Entity>, Vec<String>, Vec<SyntaxError>) {
    let Some(tree) = parse_source(tree_sitter_qmljs::LANGUAGE.into(), source) else {
        return (vec![], vec![], vec![]);
    };
    let mut entities = Vec::new();
    let mut cursor = tree.root_node().walk();
    for child in tree.root_node().children(&mut cursor) {
        if child.kind() == "ui_object_definition"
            && let Some(e) = parse_object(child, source, loc_map)
        {
            entities.push(e);
        }
    }
    let imports = extract_imports_from_tree(&tree, source, &["ui_import"]);
    let errors = detect_syntax_errors(&tree);
    (entities, imports, errors)
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
    let (start_line, end_line) = node_lines(node);
    Some(Entity {
        name,
        entity_type: EntityType::Class,
        loc: loc_map.count(start_line, end_line),
        start_line,
        end_line,
        children: methods,
    })
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
    let src = source.as_bytes();
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_owned)?;
    let (start_line, end_line) = node_lines(node);
    Some(Entity::new(
        name,
        EntityType::Method,
        loc_map.count(start_line, end_line),
        start_line,
        end_line,
    ))
}

pub(super) fn resolve_imports(
    imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim().strip_prefix("import ")?.trim().trim_matches('"');
            if raw.starts_with("./") || raw.starts_with("../") {
                Some(Dependency { name: raw.to_owned(), kind: DependencyKind::Internal })
            } else {
                let name = raw.split_whitespace().next()?.to_owned();
                Some(Dependency { name, kind: DependencyKind::External })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;

    fn entities(s: &str) -> Vec<Entity> { parse_all(s, &LocMap::build(s)).0 }
    fn imports(s: &str) -> Vec<String> { parse_all(s, &LocMap::build(s)).1 }

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
}
