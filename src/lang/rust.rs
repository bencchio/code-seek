use std::path::Path;

use tree_sitter::Node;

use super::{Context, LocMap, body_children, collect_children, container_entity, leaf_entity, name_field};
use crate::model::{Dependency, DependencyKind, Entity, EntityType};

pub(super) fn parse_node(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context, depth: usize) -> Option<Entity> {
    let src = source.as_bytes();
    match node.kind() {
        "function_item" | "function_signature_item" => {
            let entity_type = if context == Context::TypeBody { EntityType::Method } else { EntityType::Function };
            Some(leaf_entity(node, name_field(node, src)?, entity_type, loc_map))
        }
        "struct_item" => Some(leaf_entity(node, name_field(node, src)?, EntityType::Struct, loc_map)),
        "enum_item" => Some(leaf_entity(node, name_field(node, src)?, EntityType::Enum, loc_map)),
        "impl_item" => {
            let name = impl_target_type(node, src)?;
            let children = body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(node, name, EntityType::Impl, loc_map, children))
        }
        "trait_item" => {
            let name = name_field(node, src)?;
            let children = body_children(node, source, loc_map, Context::TypeBody, depth, parse_node);
            Some(container_entity(node, name, EntityType::Trait, loc_map, children))
        }
        "mod_item" => {
            // `mod foo;` has no body and declares nothing to show here
            let body = node.child_by_field_name("body")?;
            let name = name_field(node, src)?;
            let children = collect_children(body, source, loc_map, Context::TopLevel, depth + 1, parse_node);
            Some(container_entity(node, name, EntityType::Namespace, loc_map, children))
        }
        _ => None,
    }
}

fn first_crate_segment(import: &str) -> Option<String> {
    let s = import
        .trim_start_matches("use ")
        .trim_start_matches("extern crate ")
        .trim()
        .split([':', ' ', ';'])
        .next()?
        .to_owned();
    if s.is_empty() { None } else { Some(s) }
}

fn is_std_crate(crate_name: &str) -> bool {
    matches!(crate_name, "std" | "core" | "alloc" | "proc_macro" | "test")
}

pub(super) fn resolve_imports(
    imports: &[String],
    project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.starts_with("use crate::") {
                Some(Dependency {
                    name: raw
                        .trim_start_matches("use ")
                        .trim_start_matches("extern crate ")
                        .trim_end_matches(';')
                        .to_owned(),
                    kind: DependencyKind::Internal,
                })
            } else if let Some(crate_name) = first_crate_segment(raw) {
                let kind = if is_std_crate(&crate_name) {
                    DependencyKind::External
                } else if raw.starts_with("use ") || raw.starts_with("extern crate ") {
                    let candidate_rs = Path::new("src").join(format!("{crate_name}.rs"));
                    let candidate_mod = Path::new("src").join(&crate_name).join("mod.rs");
                    if project_files.iter().any(|p| p.ends_with(&candidate_rs) || p.ends_with(&candidate_mod)) {
                        DependencyKind::Internal
                    } else {
                        DependencyKind::External
                    }
                } else {
                    DependencyKind::External
                };
                Some(Dependency { name: crate_name, kind })
            } else {
                None
            }
        })
        .collect()
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

    fn parse(src: &str) -> Vec<Entity> { crate::lang::test_parse("Rust", src).entities }
    fn imports(src: &str) -> Vec<String> { crate::lang::test_parse("Rust", src).imports }

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

    #[test]
    fn extracts_use_declarations() {
        let src = "use std::collections::HashMap;\nuse serde::Serialize;\nfn main() {}";
        let result = imports(src);
        assert_eq!(result.len(), 2);
        assert!(result[0].contains("HashMap"));
        assert!(result[1].contains("serde"));
    }

    #[test]
    fn extracts_extern_crate() {
        let src = "extern crate serde;\nfn main() {}";
        let result = imports(src);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("serde"));
    }

    #[test]
    fn resolves_use_declarations() {
        let imports = vec![
            "use std::collections::HashMap;".into(),
            "use crate::util::helper;".into(),
            "use serde::Serialize;".into(),
        ];
        let project = std::collections::HashSet::from([
            std::path::PathBuf::from("src/util.rs"),
            std::path::PathBuf::from("src/main.rs"),
        ]);
        let deps = resolve_imports(&imports, &project);
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "std");
        assert_eq!(deps[0].kind, DependencyKind::External);
        assert_eq!(deps[1].name, "crate::util::helper");
        assert_eq!(deps[1].kind, DependencyKind::Internal);
        assert_eq!(deps[2].name, "serde");
        assert_eq!(deps[2].kind, DependencyKind::External);
    }

    #[test]
    fn resolves_extern_crate() {
        let imports = vec!["extern crate serde;".into()];
        let project = std::collections::HashSet::new();
        let deps = resolve_imports(&imports, &project);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "serde");
        assert_eq!(deps[0].kind, DependencyKind::External);
    }
}
