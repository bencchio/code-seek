use tree_sitter::Node;

use super::{Context, LocMap, body_children, container_entity, js, leaf_entity, name_field};
use crate::model::{Entity, EntityType};

/// TypeScript-only node kinds; everything else (functions, classes,
/// methods, arrow functions, `export` wrappers) falls through to the
/// shared JavaScript base. Type aliases are skipped: no matching entity type.
pub(super) fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "abstract_class_declaration" => {
            let name = name_field(node, src)?;
            let children =
                body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(
                node,
                name,
                EntityType::Class,
                loc_map,
                children,
            ))
        }
        "interface_declaration" => {
            let name = name_field(node, src)?;
            let children =
                body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(
                node,
                name,
                EntityType::Trait,
                loc_map,
                children,
            ))
        }
        "enum_declaration" => Some(leaf_entity(
            node,
            name_field(node, src)?,
            EntityType::Enum,
            loc_map,
        )),
        // `namespace X {}` parses as expression_statement > internal_module
        "expression_statement" => {
            let child = node.named_child(0)?;
            if child.kind() == "internal_module" {
                parse_node(child, source, loc_map, context, depth)
            } else {
                None
            }
        }
        "internal_module" => {
            let name = name_field(node, src)?;
            let children =
                body_children(node, source, loc_map, Context::TopLevel, depth, parse_node);
            Some(container_entity(
                node,
                name,
                EntityType::Namespace,
                loc_map,
                children,
            ))
        }
        "method_signature" | "abstract_method_signature" if context == Context::TypeBody => Some(
            leaf_entity(node, name_field(node, src)?, EntityType::Method, loc_map),
        ),
        _ => js::parse_common(node, source, loc_map, context, depth, parse_node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dependency, DependencyKind};

    fn parse(src: &str) -> Vec<Entity> {
        crate::lang::test_parse("TypeScript", src).entities
    }
    fn imports(src: &str) -> Vec<String> {
        crate::lang::test_parse("TypeScript", src).imports
    }
    fn resolve(raw: &[String]) -> Vec<Dependency> {
        crate::lang::resolve_imports("TypeScript", raw, &Default::default())
    }

    #[test]
    fn parses_function_with_type_annotations() {
        let entities = parse("function greet(name: string): string { return 'Hello ' + name; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "greet");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_class_with_methods() {
        let src = "class Animal {\n  private name: string;\n  constructor(name: string) { this.name = name; }\n  speak(): string { return this.name; }\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Animal");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 2);
        assert!(
            entities[0]
                .children
                .iter()
                .all(|c| c.entity_type == EntityType::Method)
        );
        assert!(entities[0].children.iter().any(|c| c.name == "constructor"));
        assert!(entities[0].children.iter().any(|c| c.name == "speak"));
    }

    #[test]
    fn parses_abstract_class() {
        let src = "abstract class Shape {\n  abstract area(): number;\n  describe(): string { return 'shape'; }\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Shape");
        assert_eq!(entities[0].entity_type, EntityType::Class);
        assert_eq!(entities[0].children.len(), 2);
        assert!(entities[0].children.iter().any(|c| c.name == "area"));
        assert!(entities[0].children.iter().any(|c| c.name == "describe"));
    }

    #[test]
    fn parses_interface_as_trait() {
        let src =
            "interface Serializer {\n  serialize(): string;\n  deserialize(data: string): void;\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Serializer");
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
    fn parses_enum() {
        let entities = parse("enum Color {\n  Red,\n  Green,\n  Blue,\n}");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Color");
        assert_eq!(entities[0].entity_type, EntityType::Enum);
    }

    #[test]
    fn parses_namespace_with_children() {
        let src =
            "namespace Utils {\n  export function helper(): void {}\n  export class Tool {}\n}";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Utils");
        assert_eq!(entities[0].entity_type, EntityType::Namespace);
        assert_eq!(entities[0].children.len(), 2);
        assert!(
            entities[0]
                .children
                .iter()
                .any(|c| c.name == "helper" && c.entity_type == EntityType::Function)
        );
        assert!(
            entities[0]
                .children
                .iter()
                .any(|c| c.name == "Tool" && c.entity_type == EntityType::Class)
        );
    }

    #[test]
    fn parses_typed_arrow_function_const() {
        let entities = parse("const add = (a: number, b: number): number => a + b;");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "add");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_export_function() {
        let entities = parse("export function compute(x: number): number { return x * 2; }");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "compute");
        assert_eq!(entities[0].entity_type, EntityType::Function);
    }

    #[test]
    fn parses_export_interface() {
        let entities = parse("export interface Config {\n  load(): void;\n}");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Config");
        assert_eq!(entities[0].entity_type, EntityType::Trait);
    }

    #[test]
    fn skips_type_alias() {
        let entities = parse("type Alias = string | number;");
        assert!(entities.is_empty());
    }

    #[test]
    fn collects_import_statements() {
        let src = "import React from 'react';\nimport { foo } from \"./utils\";\nimport type { Bar } from './types';";
        let raw = imports(src);
        assert_eq!(raw.len(), 3);
        assert!(raw.iter().any(|i| i.contains("react")));
        assert!(raw.iter().any(|i| i.contains("./utils")));
        assert!(raw.iter().any(|i| i.contains("./types")));
    }

    #[test]
    fn resolve_imports_classifies_relative_as_internal() {
        let raw = vec!["import foo from './bar'".to_owned()];
        let deps = resolve(&raw);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "./bar");
        assert_eq!(deps[0].kind, DependencyKind::Internal);
    }

    #[test]
    fn resolve_imports_classifies_package_as_external() {
        let raw = vec!["import React from 'react'".to_owned()];
        let deps = resolve(&raw);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "react");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }

    #[test]
    fn detects_syntax_error() {
        let src = "function foo( { return; }";
        let errors = crate::lang::test_parse("TypeScript", src).errors;
        assert!(!errors.is_empty());
    }

    #[test]
    fn multiple_top_level_entities() {
        let src = "function alpha() {}\nconst beta = () => {}\nclass Gamma {}\ninterface Delta {}\nenum Epsilon {}\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 5);
        assert!(
            entities
                .iter()
                .any(|e| e.name == "alpha" && e.entity_type == EntityType::Function)
        );
        assert!(
            entities
                .iter()
                .any(|e| e.name == "beta" && e.entity_type == EntityType::Function)
        );
        assert!(
            entities
                .iter()
                .any(|e| e.name == "Gamma" && e.entity_type == EntityType::Class)
        );
        assert!(
            entities
                .iter()
                .any(|e| e.name == "Delta" && e.entity_type == EntityType::Trait)
        );
        assert!(
            entities
                .iter()
                .any(|e| e.name == "Epsilon" && e.entity_type == EntityType::Enum)
        );
    }
}
