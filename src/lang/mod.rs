use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Parser};

use crate::model::{Dependency, DependencyKind, Entity, EntityType, FileResult, SyntaxError};

pub(crate) mod c;
mod c_family;
pub(crate) mod cpp;
pub(crate) mod elixir;
pub(crate) mod go;
pub(crate) mod js;
mod loc;
pub(crate) mod python;
pub(crate) mod qml;
pub(crate) mod rust;
pub(crate) mod ts;

pub(crate) use loc::LocMap;

/// Where a node sits while walking: at file/namespace level or inside a
/// type body (class, struct, trait, impl). Parsers use it to decide
/// Function vs Method and to reject members outside their container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    TopLevel,
    TypeBody,
}

pub(super) type ParseNode = fn(Node<'_>, &str, &LocMap, Context, usize) -> Option<Entity>;
type ResolveImports = fn(&[String], &HashSet<PathBuf>) -> Vec<Dependency>;

#[derive(Default)]
pub(super) struct ParseOutput {
    pub(super) entities: Vec<Entity>,
    pub(super) imports: Vec<String>,
    pub(super) errors: Vec<SyntaxError>,
}

pub(crate) struct LangDef {
    pub(crate) canonical: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) extensions: &'static [&'static str],
    pub(crate) icon: &'static str,
    grammar: fn() -> tree_sitter::Language,
    import_kinds: &'static [&'static str],
    parse_node: ParseNode,
    resolve_imports: ResolveImports,
}

pub(crate) static LANGUAGES: &[LangDef] = &[
    LangDef {
        canonical: "C",
        aliases: &["c"],
        extensions: &["c", "h"],
        icon: "󰙱",
        grammar: || tree_sitter_c::LANGUAGE.into(),
        import_kinds: &["preproc_include"],
        parse_node: c::parse_node,
        resolve_imports: c_family::resolve_includes,
    },
    LangDef {
        canonical: "C++",
        aliases: &["cpp", "c++"],
        extensions: &["cpp", "cc", "cxx", "hpp", "hxx", "h++"],
        icon: "󰙲",
        grammar: || tree_sitter_cpp::LANGUAGE.into(),
        import_kinds: &["preproc_include"],
        parse_node: cpp::parse_node,
        resolve_imports: c_family::resolve_includes,
    },
    LangDef {
        canonical: "Elixir",
        aliases: &["elixir", "ex"],
        extensions: &["ex", "exs"],
        icon: "",
        grammar: || tree_sitter_elixir::LANGUAGE.into(),
        import_kinds: &[],
        parse_node: elixir::parse_node,
        resolve_imports: elixir::resolve_imports,
    },
    LangDef {
        canonical: "JavaScript",
        aliases: &["js", "javascript"],
        extensions: &["js", "mjs", "cjs"],
        icon: "󰌞",
        grammar: || tree_sitter_javascript::LANGUAGE.into(),
        import_kinds: &["import_statement"],
        parse_node: js::parse_node,
        resolve_imports: js::resolve_imports,
    },
    LangDef {
        canonical: "Python",
        aliases: &["python", "py"],
        extensions: &["py", "pyw"],
        icon: "󰌠",
        grammar: || tree_sitter_python::LANGUAGE.into(),
        import_kinds: &["import_statement", "import_from_statement"],
        parse_node: python::parse_node,
        resolve_imports: python::resolve_imports,
    },
    LangDef {
        canonical: "QML",
        aliases: &["qml"],
        extensions: &["qml"],
        icon: "󰈚",
        grammar: || tree_sitter_qmljs::LANGUAGE.into(),
        import_kinds: &["ui_import"],
        parse_node: qml::parse_node,
        resolve_imports: qml::resolve_imports,
    },
    LangDef {
        canonical: "Rust",
        aliases: &["rust"],
        extensions: &["rs"],
        icon: "󱘗",
        grammar: || tree_sitter_rust::LANGUAGE.into(),
        import_kinds: &["use_declaration", "extern_crate_declaration"],
        parse_node: rust::parse_node,
        resolve_imports: rust::resolve_imports,
    },
    LangDef {
        canonical: "Go",
        aliases: &["go", "golang"],
        extensions: &["go"],
        icon: "󰟓",
        grammar: || tree_sitter_go::LANGUAGE.into(),
        import_kinds: &["import_declaration"],
        parse_node: go::parse_node,
        resolve_imports: go::resolve_imports,
    },
    LangDef {
        canonical: "TypeScript",
        aliases: &["ts", "typescript"],
        extensions: &["ts", "mts", "cts"],
        icon: "󰛦",
        grammar: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        import_kinds: &["import_statement"],
        parse_node: ts::parse_node,
        resolve_imports: js::resolve_imports,
    },
];

fn find(canonical: &str) -> Option<&'static LangDef> {
    LANGUAGES.iter().find(|l| l.canonical == canonical)
}

pub(crate) fn detect(path: &Path) -> Option<&'static str> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    LANGUAGES
        .iter()
        .find(|l| l.extensions.contains(&ext))
        .map(|l| l.canonical)
}

pub(crate) fn parse_file(path: &Path, source: &str, language: &'static str) -> FileResult {
    let loc_map = LocMap::build(source);
    let loc = loc_map.total();
    let output = find(language)
        .map(|def| run_parser(def, source, &loc_map))
        .unwrap_or_default();
    FileResult {
        path: path.to_path_buf(),
        language,
        loc,
        entities: output.entities,
        imports: output.imports,
        dependencies: vec![],
        errors: output.errors,
    }
}

pub(crate) fn resolve_imports(
    language: &str,
    imports: &[String],
    project_files: &HashSet<PathBuf>,
) -> Vec<Dependency> {
    find(language)
        .map(|def| (def.resolve_imports)(imports, project_files))
        .unwrap_or_default()
}

/// Single parse pipeline shared by all languages: parse once, then extract
/// entities, imports, and syntax errors from the same tree.
fn run_parser(def: &LangDef, source: &str, loc_map: &LocMap) -> ParseOutput {
    let Some(tree) = parse_source((def.grammar)(), source) else {
        return ParseOutput::default();
    };
    let entities = collect_children(
        tree.root_node(),
        source,
        loc_map,
        Context::TopLevel,
        0,
        def.parse_node,
    );
    let imports = extract_imports_from_tree(&tree, source, def.import_kinds);
    let errors = detect_syntax_errors(&tree);
    ParseOutput {
        entities,
        imports,
        errors,
    }
}

#[cfg(test)]
pub(super) fn test_parse(canonical: &str, source: &str) -> ParseOutput {
    let def = find(canonical).expect("language not registered");
    run_parser(def, source, &LocMap::build(source))
}

pub(super) fn parse_source(
    language: tree_sitter::Language,
    source: &str,
) -> Option<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(source.as_bytes(), None)
}

pub(super) fn extract_imports_from_tree(
    tree: &tree_sitter::Tree,
    source: &str,
    import_kinds: &[&str],
) -> Vec<String> {
    let mut cursor = tree.root_node().walk();
    let mut imports = Vec::new();
    for child in tree.root_node().children(&mut cursor) {
        if import_kinds.contains(&child.kind()) {
            match child.utf8_text(source.as_bytes()) {
                Ok(text) => imports.push(text.trim().to_owned()),
                Err(_) => crate::log::warn("failed to extract import text (non-UTF-8 content)"),
            }
        }
    }
    imports
}

pub(super) fn collect_children<F>(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
    f: F,
) -> Vec<Entity>
where
    F: Fn(Node<'_>, &str, &LocMap, Context, usize) -> Option<Entity>,
{
    if depth >= 64 {
        return Vec::new();
    }
    let mut entities = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(e) = f(child, source, loc_map, context, depth) {
            entities.push(e);
        }
    }
    entities.sort_by(|a, b| a.name.cmp(&b.name));
    entities
}

pub(super) fn node_lines(node: Node<'_>) -> (usize, usize) {
    (node.start_position().row + 1, node.end_position().row + 1)
}

/// Text of the node's `name` field, the common naming shape across grammars.
pub(super) fn name_field(node: Node<'_>, src: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_owned)
}

pub(super) fn leaf_entity(
    node: Node<'_>,
    name: String,
    entity_type: EntityType,
    loc_map: &LocMap,
) -> Entity {
    let (start_line, end_line) = node_lines(node);
    Entity::new(
        name,
        entity_type,
        loc_map.count(start_line, end_line),
        start_line,
        end_line,
    )
}

pub(super) fn container_entity(
    node: Node<'_>,
    name: String,
    entity_type: EntityType,
    loc_map: &LocMap,
    children: Vec<Entity>,
) -> Entity {
    let (start_line, end_line) = node_lines(node);
    Entity {
        name,
        entity_type,
        kind: String::new(),
        loc: loc_map.count(start_line, end_line),
        start_line,
        end_line,
        children,
    }
}

/// Entities of the node's `body` field; empty when the node has no body.
pub(super) fn body_children<F>(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: Context,
    depth: usize,
    f: F,
) -> Vec<Entity>
where
    F: Fn(Node<'_>, &str, &LocMap, Context, usize) -> Option<Entity>,
{
    node.child_by_field_name("body")
        .map(|body| collect_children(body, source, loc_map, context, depth + 1, f))
        .unwrap_or_default()
}

/// Path-based import classification: names starting with one of
/// `internal_prefixes` are project-internal, everything else is a package.
pub(super) fn classify_path(name: String, internal_prefixes: &[&str]) -> Dependency {
    let kind = if internal_prefixes.iter().any(|p| name.starts_with(p)) {
        DependencyKind::Internal
    } else {
        DependencyKind::External
    };
    Dependency { name, kind }
}

pub(super) fn declarator_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "namespace_identifier" => {
            node.utf8_text(src).ok().map(str::to_owned)
        }
        "qualified_identifier" => node
            .child_by_field_name("name")
            .and_then(|n| declarator_name(n, src)),
        "function_declarator" | "pointer_declarator" | "reference_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|n| declarator_name(n, src)),
        _ => None,
    }
}

pub(super) fn detect_syntax_errors(tree: &tree_sitter::Tree) -> Vec<SyntaxError> {
    let mut errors = Vec::new();
    walk_errors(tree.root_node(), &mut errors);
    errors
}

fn walk_errors(node: tree_sitter::Node<'_>, errors: &mut Vec<SyntaxError>) {
    if node.is_error() {
        errors.push(SyntaxError {
            kind: "error".into(),
            node_kind: node.kind().into(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        });
        return;
    }
    if node.is_missing() {
        errors.push(SyntaxError {
            kind: "missing".into(),
            node_kind: node.kind().into(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        });
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_errors(child, errors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detect_c_extensions() {
        assert_eq!(detect(Path::new("foo.c")), Some("C"));
        assert_eq!(detect(Path::new("foo.h")), Some("C"));
    }

    #[test]
    fn detect_cpp_extensions() {
        assert_eq!(detect(Path::new("foo.cpp")), Some("C++"));
        assert_eq!(detect(Path::new("foo.cc")), Some("C++"));
        assert_eq!(detect(Path::new("foo.hpp")), Some("C++"));
    }

    #[test]
    fn detect_elixir() {
        assert_eq!(detect(Path::new("foo.ex")), Some("Elixir"));
        assert_eq!(detect(Path::new("foo.exs")), Some("Elixir"));
    }

    #[test]
    fn detect_empty_extension() {
        assert_eq!(detect(Path::new("foo.")), None);
    }

    #[test]
    fn detect_error_node() {
        let src = "fn main() { let x = }\n";
        let tree = parse_source(tree_sitter_rust::LANGUAGE.into(), src).unwrap();
        let errors = detect_syntax_errors(&tree);
        assert!(!errors.is_empty());
        let e = errors
            .iter()
            .find(|e| e.kind == "error")
            .expect("should have error kind");
        assert_eq!(e.node_kind, "ERROR");
        assert_eq!(e.start_line, 1);
        assert_eq!(e.end_line, 1);
    }

    #[test]
    fn detect_go() {
        assert_eq!(detect(Path::new("foo.go")), Some("Go"));
    }

    #[test]
    fn detect_javascript() {
        assert_eq!(detect(Path::new("foo.js")), Some("JavaScript"));
        assert_eq!(detect(Path::new("foo.mjs")), Some("JavaScript"));
        assert_eq!(detect(Path::new("foo.cjs")), Some("JavaScript"));
    }

    #[test]
    fn detect_missing_node() {
        let src = "int x = 1\nint y = 2;\n";
        let tree = parse_source(tree_sitter_c::LANGUAGE.into(), src).unwrap();
        let errors = detect_syntax_errors(&tree);
        assert!(!errors.is_empty());
        let e = errors
            .iter()
            .find(|e| e.kind == "missing")
            .expect("should have missing kind");
        assert_eq!(e.start_line, 1);
        assert!(!e.node_kind.is_empty());
    }

    #[test]
    fn detect_path_without_extension() {
        assert_eq!(detect(Path::new("Makefile")), None);
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect(Path::new("foo.py")), Some("Python"));
        assert_eq!(detect(Path::new("foo.pyw")), Some("Python"));
    }

    #[test]
    fn detect_qml() {
        assert_eq!(detect(Path::new("foo.qml")), Some("QML"));
    }

    #[test]
    fn detect_rust() {
        assert_eq!(detect(Path::new("foo.rs")), Some("Rust"));
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(detect(Path::new("foo.ts")), Some("TypeScript"));
        assert_eq!(detect(Path::new("foo.mts")), Some("TypeScript"));
        assert_eq!(detect(Path::new("foo.cts")), Some("TypeScript"));
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect(Path::new("foo.tsx")), None);
        assert_eq!(detect(Path::new("foo")), None);
        assert_eq!(detect(Path::new("Makefile")), None);
    }

    #[test]
    fn every_language_runs_through_the_generic_driver() {
        for def in LANGUAGES {
            let out = run_parser(def, "", &LocMap::build(""));
            assert!(
                out.entities.is_empty(),
                "{}: empty source yields no entities",
                def.canonical
            );
            assert!(
                out.imports.is_empty(),
                "{}: empty source yields no imports",
                def.canonical
            );
        }
    }
}
