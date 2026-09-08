//! Shared base for the C and C++ parsers: `#include` resolution,
//! `function_definition` handling, and `declaration`/`type_definition`
//! unwrapping are identical in both grammars.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use super::{Context, LocMap, ParseNode, declarator_name, leaf_entity};
use crate::model::{Dependency, DependencyKind, Entity, EntityType};

pub(super) fn function_entity(node: Node<'_>, source: &str, loc_map: &LocMap, context: Context) -> Option<Entity> {
    let name = declarator_name(node.child_by_field_name("declarator")?, source.as_bytes())?;
    let entity_type = if context == Context::TypeBody { EntityType::Method } else { EntityType::Function };
    Some(leaf_entity(node, name, entity_type, loc_map))
}

/// `declaration` and `type_definition` wrap the specifier they declare
/// (e.g. `typedef struct Foo {...} FooAlias;`); re-parse the first child
/// matching `kinds` with the language's own `parse_node`.
pub(super) fn unwrap_declaration(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
    kinds: &[&str],
    parse_node: ParseNode,
) -> Option<Entity> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if kinds.contains(&child.kind()) {
            return parse_node(child, source, loc_map, context, depth);
        }
    }
    None
}

pub(super) fn resolve_includes(
    imports: &[String],
    project_files: &HashSet<PathBuf>,
) -> Vec<Dependency> {
    imports
        .iter()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.starts_with("#include") {
                let inner = raw.trim_start_matches("#include").trim();
                let (content, is_angle) = if inner.starts_with('<') {
                    (inner.trim_matches(|c| c == '<' || c == '>'), true)
                } else if inner.starts_with('"') {
                    (inner.trim_matches('"'), false)
                } else {
                    return None;
                };
                let name = content.to_owned();
                let kind = if is_angle {
                    DependencyKind::External
                } else {
                    let file_path = Path::new(&name);
                    if project_files.iter().any(|p| p.ends_with(file_path)) {
                        DependencyKind::Internal
                    } else {
                        DependencyKind::External
                    }
                };
                Some(Dependency { name, kind })
            } else {
                None
            }
        })
        .collect()
}
