use tree_sitter::Node;

use super::{Context, LocMap, body_children, classify_path, container_entity, leaf_entity, name_field};
use crate::model::{Dependency, DependencyKind, Entity, EntityType};

pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context, depth: usize) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_definition" | "async_function_definition" => {
            let entity_type = if context == Context::TypeBody { EntityType::Method } else { EntityType::Function };
            Some(leaf_entity(node, name_field(node, src)?, entity_type, loc_map))
        }
        "class_definition" => {
            let name = name_field(node, src)?;
            let children = body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(node, name, EntityType::Class, loc_map, children))
        }
        "decorated_definition" => {
            node.child_by_field_name("definition")
                .and_then(|def| parse_node(def, source, loc_map, context, depth))
        }
        _ => None,
    }
}

pub(super) fn resolve_imports(
    imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .flat_map(|raw| parse_import(raw.trim()))
        .collect()
}

fn parse_import(raw: &str) -> Vec<Dependency> {
    if let Some(rest) = raw.strip_prefix("from ") {
        // "from MODULE import ..." — module is everything before " import "
        let module = rest.split(" import ").next().unwrap_or("").trim().to_owned();
        if module.is_empty() {
            return vec![];
        }
        vec![classify_path(module, &["."])]
    } else if let Some(rest) = raw.strip_prefix("import ") {
        // "import MODULE, MODULE2 as ALIAS, ..."
        rest.split(',')
            .filter_map(|segment| {
                let name = segment.split(" as ").next().unwrap_or("").trim().to_owned();
                if name.is_empty() { None } else { Some(Dependency { name, kind: DependencyKind::External }) }
            })
            .collect()
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Entity> { crate::lang::test_parse("Python", src).entities }
    fn imports(src: &str) -> Vec<String> { crate::lang::test_parse("Python", src).imports }

    #[test]
    fn parses_function_def() {
        let entities = parse("def greet(name):\n    return name\n");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "greet");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_async_function() {
        let entities = parse("async def fetch(url):\n    pass\n");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "fetch");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_class_with_methods() {
        let src = "class Animal:\n    def __init__(self, name):\n        self.name = name\n    def speak(self):\n        pass\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Animal");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 2);
        assert!(entities[0].children.iter().all(|c| c.entity_type == EntityType::Method));
        assert!(entities[0].children.iter().any(|c| c.name == "__init__"));
        assert!(entities[0].children.iter().any(|c| c.name == "speak"));
    }

    #[test]
    fn parses_decorated_function() {
        let src = "@staticmethod\ndef helper():\n    pass\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "helper");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_decorated_class() {
        let src = "@dataclass\nclass Point:\n    def origin(self):\n        pass\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Point");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 1);
    }

    #[test]
    fn collects_import_and_from_statements() {
        let src = "import os\nfrom pathlib import Path\n";
        let raw = imports(src);
        assert_eq!(raw.len(), 2);
        assert!(raw.iter().any(|i| i.contains("import os")));
        assert!(raw.iter().any(|i| i.contains("pathlib")));
    }

    #[test]
    fn resolve_imports_relative_is_internal() {
        let raw = vec!["from .utils import helper".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, ".utils");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolve_imports_bare_relative_is_internal() {
        let raw = vec!["from . import utils".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, ".");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolve_imports_package_is_external() {
        let raw = vec!["from pathlib import Path".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "pathlib");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }

    #[test]
    fn resolve_imports_multi_import() {
        let raw = vec!["import os, sys, re".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "os"));
        assert!(deps.iter().any(|d| d.name == "sys"));
        assert!(deps.iter().any(|d| d.name == "re"));
        assert!(deps.iter().all(|d| d.kind == DependencyKind::External));
    }

    #[test]
    fn resolve_imports_alias_stripped() {
        let raw = vec!["import numpy as np".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "numpy");
    }

    #[test]
    fn detects_syntax_error() {
        let src = "def foo(\n";
        let errors = crate::lang::test_parse("Python", src).errors;
        assert!(!errors.is_empty());
    }
}
