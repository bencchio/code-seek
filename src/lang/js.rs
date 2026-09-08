use tree_sitter::Node;

use super::{Context, LocMap, ParseNode, body_children, classify_path, container_entity, leaf_entity, name_field};
use crate::model::{Dependency, Entity, EntityType};

pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context, depth: usize) -> Option<Entity> {
    parse_common(node, source, loc_map, context, depth, parse_node)
}

/// Node handling shared by JavaScript and TypeScript. `recurse` is the
/// language's full `parse_node`, so wrappers (`export`, class bodies)
/// re-enter language-specific cases.
pub(super) fn parse_common(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
    recurse: ParseNode,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_declaration" => {
            Some(leaf_entity(node, name_field(node, src)?, EntityType::Function, loc_map))
        }
        "class_declaration" => {
            let name = name_field(node, src)?;
            let children = body_children(node, source, loc_map, Context::TypeBody, depth, recurse);
            Some(container_entity(node, name, EntityType::Class, loc_map, children))
        }
        "method_definition" if context == Context::TypeBody => {
            Some(leaf_entity(node, name_field(node, src)?, EntityType::Method, loc_map))
        }
        "lexical_declaration" | "variable_declaration" if context == Context::TopLevel => {
            arrow_from_declarator(node, src, loc_map)
        }
        "export_statement" => {
            node.child_by_field_name("declaration")
                .and_then(|decl| recurse(decl, source, loc_map, context, depth))
        }
        _ => None,
    }
}

fn arrow_from_declarator(node: Node<'_>, src: &[u8], loc_map: &LocMap) -> Option<Entity> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let value = child.child_by_field_name("value")?;
            if value.kind() == "arrow_function" {
                let name = name_field(child, src)?;
                return Some(leaf_entity(node, name, EntityType::Function, loc_map));
            }
        }
    }
    None
}

pub(super) fn resolve_imports(
    imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let specifier = extract_specifier(raw)?;
            Some(classify_path(specifier, &["./", "../"]))
        })
        .collect()
}

fn extract_specifier(raw: &str) -> Option<String> {
    // Find the module path in the quoted token of the import statement.
    // Handles both single and double quotes: import foo from 'bar' / "bar"
    // We scan for the first opening quote; the specifier follows immediately.
    for quote in ['"', '\''] {
        if let Some(start) = raw.find(quote) {
            let after = &raw[start + 1..];
            if let Some(end) = after.find(quote) {
                let s = &after[..end];
                if !s.is_empty() {
                    return Some(s.to_owned());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DependencyKind;

    fn parse(src: &str) -> Vec<Entity> { crate::lang::test_parse("JavaScript", src).entities }
    fn imports(src: &str) -> Vec<String> { crate::lang::test_parse("JavaScript", src).imports }

    #[test]
    fn parses_function_declaration() {
        let entities = parse("function greet(name) { return 'Hello ' + name; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "greet");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_class_with_methods() {
        let src = "class Animal {\n  constructor(name) { this.name = name; }\n  speak() { return this.name; }\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Animal");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 2);
        assert!(entities[0].children.iter().all(|c| c.entity_type == EntityType::Method));
        assert!(entities[0].children.iter().any(|c| c.name == "constructor"));
        assert!(entities[0].children.iter().any(|c| c.name == "speak"));
    }

    #[test]
    fn parses_arrow_function_const() {
        let entities = parse("const add = (a, b) => a + b;");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_export_function() {
        let entities = parse("export function compute(x) { return x * 2; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "compute");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_export_class() {
        let src = "export class Shape {\n  area() { return 0; }\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Shape");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].name, "area");
    }

    #[test]
    fn collects_import_statements() {
        let src = "import React from 'react';\nimport { foo } from \"./utils\";";
        let raw = imports(src);
        assert_eq!(raw.len(), 2);
        assert!(raw.iter().any(|i| i.contains("react")));
        assert!(raw.iter().any(|i| i.contains("./utils")));
    }

    #[test]
    fn resolve_imports_classifies_relative_as_internal() {
        let raw = vec!["import foo from './bar'".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "./bar");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolve_imports_classifies_package_as_external() {
        let raw = vec!["import React from 'react'".to_owned()];
        let deps = resolve_imports(&raw, &Default::default());
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "react");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }

    #[test]
    fn detects_syntax_error() {
        let src = "function foo( { return; }";
        let errors = crate::lang::test_parse("JavaScript", src).errors;
        assert!(!errors.is_empty());
    }

    #[test]
    fn multiple_top_level_entities() {
        let src = "function alpha() {}\nconst beta = () => {}\nclass Gamma {}\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 3);
        assert!(entities.iter().any(|e| e.name == "alpha" && e.entity_type == EntityType::Function));
        assert!(entities.iter().any(|e| e.name == "beta" && e.entity_type == EntityType::Function));
        assert!(entities.iter().any(|e| e.name == "Gamma" && e.entity_type == EntityType::Class));
    }
}
