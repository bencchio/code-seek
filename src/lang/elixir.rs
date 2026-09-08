use tree_sitter::Node;

use super::{Context, LocMap, collect_children, container_entity, leaf_entity};
use crate::model::{Dependency, Entity, EntityType};

pub(super) fn parse_node(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
) -> Option<Entity> {
    if node.kind() != "call" {
        return None;
    }
    let src = source.as_bytes();
    let target = call_target(node, src)?;
    match target.as_str() {
        "defmodule" => {
            let name = module_name(node, src)?;
            let mut children = block_children(node, source, loc_map, Context::TopLevel, depth);
            for child in &mut children {
                if child.kind == "defstruct" {
                    child.name = name.clone();
                }
            }
            Some(with_kind(
                container_entity(node, name, EntityType::Namespace, loc_map, children),
                target,
            ))
        }
        "defprotocol" => {
            let name = module_name(node, src)?;
            let children = block_children(node, source, loc_map, Context::TypeBody, depth);
            Some(with_kind(
                container_entity(node, name, EntityType::Trait, loc_map, children),
                target,
            ))
        }
        "defimpl" => {
            let name = impl_name(node, src)?;
            let children = block_children(node, source, loc_map, Context::TypeBody, depth);
            Some(with_kind(
                container_entity(node, name, EntityType::Impl, loc_map, children),
                target,
            ))
        }
        "defstruct" => Some(with_kind(
            leaf_entity(node, "defstruct".to_owned(), EntityType::Struct, loc_map),
            target,
        )),
        "def" | "defp" | "defmacro" | "defmacrop" => {
            let entity_type = if context == Context::TypeBody {
                EntityType::Method
            } else {
                EntityType::Function
            };
            Some(with_kind(
                leaf_entity(node, def_name(node, src)?, entity_type, loc_map),
                target,
            ))
        }
        _ => None,
    }
}

pub(super) fn resolve_imports(
    _imports: &[String],
    _project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    Vec::new()
}

fn with_kind(mut entity: Entity, kind: String) -> Entity {
    entity.kind = kind;
    entity
}

fn block_children(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
) -> Vec<Entity> {
    do_block(node)
        .map(|block| collect_children(block, source, loc_map, context, depth + 1, parse_node))
        .unwrap_or_default()
}

fn call_target(node: Node<'_>, src: &[u8]) -> Option<String> {
    node.child_by_field_name("target")
        .filter(|t| t.kind() == "identifier")
        .and_then(|t| t.utf8_text(src).ok())
        .map(str::to_owned)
}

fn named_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|n| n.kind() == kind)
}

fn do_block(node: Node<'_>) -> Option<Node<'_>> {
    named_child(node, "do_block")
}

fn arguments(node: Node<'_>) -> Option<Node<'_>> {
    named_child(node, "arguments")
}

fn module_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let args = arguments(node)?;
    let mut cursor = args.walk();
    args.children(&mut cursor)
        .find(|n| n.kind() == "alias")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_owned)
}

fn impl_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let proto = module_name(node, src)?;
    match for_type(node, src) {
        Some(ty) => Some(format!("{proto} for {ty}")),
        None => Some(proto),
    }
}

fn for_type(node: Node<'_>, src: &[u8]) -> Option<String> {
    let args = arguments(node)?;
    let text = args.utf8_text(src).ok()?;
    text.split_once("for:")
        .map(|(_, rest)| rest.trim().trim_end_matches(',').trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn def_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let args = arguments(node)?;
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if let Some(name) = name_from_arg(child, src) {
            return Some(name);
        }
    }
    None
}

fn name_from_arg(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => node.utf8_text(src).ok().map(str::to_owned),
        "call" => call_target(node, src),
        "binary_operator" => node
            .child_by_field_name("left")
            .and_then(|left| name_from_arg(left, src)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Vec<Entity> {
        crate::lang::test_parse("Elixir", src).entities
    }

    #[test]
    fn empty_source() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn parses_defmodule() {
        let src = "defmodule Foo do\nend\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Foo");
        assert_eq!(entities[0].entity_type, EntityType::Namespace);
        assert_eq!(entities[0].kind, "defmodule");
    }

    #[test]
    fn parses_nested_module_name() {
        let src = "defmodule Foo.Bar do\nend\n";
        let entities = parse(src);
        assert_eq!(entities[0].name, "Foo.Bar");
    }

    #[test]
    fn parses_def_inside_module() {
        let src = "defmodule Foo do\n  def hello(name) do\n    name\n  end\nend\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].name, "hello");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Function);
        assert_eq!(entities[0].children[0].kind, "def");
    }

    #[test]
    fn parses_defp_and_macro() {
        let src =
            "defmodule Foo do\n  defp secret, do: 1\n  defmacro plus(a) do\n    a\n  end\nend\n";
        let entities = parse(src);
        let secret = entities[0]
            .children
            .iter()
            .find(|e| e.name == "secret")
            .unwrap();
        assert_eq!(secret.kind, "defp");
        let plus = entities[0]
            .children
            .iter()
            .find(|e| e.name == "plus")
            .unwrap();
        assert_eq!(plus.kind, "defmacro");
    }

    #[test]
    fn parses_guard_clause() {
        let src =
            "defmodule Foo do\n  def hello(name) when is_binary(name) do\n    name\n  end\nend\n";
        let entities = parse(src);
        assert_eq!(entities[0].children[0].name, "hello");
    }

    #[test]
    fn skips_other_calls() {
        let src = "defmodule Foo do\n  IO.puts(\"hi\")\nend\n";
        let entities = parse(src);
        assert!(entities[0].children.is_empty());
    }

    #[test]
    fn parses_protocol() {
        let src = "defprotocol Size do\n  def size(data)\nend\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Size");
        assert_eq!(entities[0].entity_type, EntityType::Trait);
        assert_eq!(entities[0].kind, "defprotocol");
        assert_eq!(entities[0].children.len(), 1);
        assert_eq!(entities[0].children[0].name, "size");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Method);
        assert_eq!(entities[0].children[0].kind, "def");
    }

    #[test]
    fn parses_impl() {
        let src = "defimpl Size, for: BitString do\n  def size(data), do: byte_size(data)\nend\n";
        let entities = parse(src);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Size for BitString");
        assert_eq!(entities[0].entity_type, EntityType::Impl);
        assert_eq!(entities[0].kind, "defimpl");
        assert_eq!(entities[0].children[0].entity_type, EntityType::Method);
    }

    #[test]
    fn parses_defstruct() {
        let src = "defmodule User do\n  defstruct [:name, :age]\nend\n";
        let entities = parse(src);
        let st = entities[0]
            .children
            .iter()
            .find(|e| e.entity_type == EntityType::Struct)
            .unwrap();
        assert_eq!(st.name, "User");
        assert_eq!(st.kind, "defstruct");
    }
}
