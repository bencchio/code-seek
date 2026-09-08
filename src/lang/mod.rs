use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::model::{Dependency, DependencyKind, Entity, FileResult, SyntaxError};

pub(crate) mod c;
pub(crate) mod cpp;
mod loc;
pub(crate) mod qml;
pub(crate) mod rust;

pub(crate) use loc::LocMap;

type ParseAll = fn(&str, &LocMap) -> (Vec<Entity>, Vec<String>, Vec<SyntaxError>);

pub(crate) struct LangDef {
    pub(crate) canonical: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) extensions: &'static [&'static str],
    pub(crate) icon: &'static str,
    parse_all: ParseAll,
}

pub(crate) static LANGUAGES: &[LangDef] = &[
    LangDef {
        canonical: "C",
        aliases: &["c"],
        extensions: &["c", "h"],
        icon: "󰙱",
        parse_all: c::parse_all,
    },
    LangDef {
        canonical: "C++",
        aliases: &["cpp", "c++"],
        extensions: &["cpp", "cc", "cxx", "hpp", "hxx", "h++"],
        icon: "󰙲",
        parse_all: cpp::parse_all,
    },
    LangDef {
        canonical: "QML",
        aliases: &["qml"],
        extensions: &["qml"],
        icon: "󰈚",
        parse_all: qml::parse_all,
    },
    LangDef {
        canonical: "Rust",
        aliases: &["rust"],
        extensions: &["rs"],
        icon: "󱘗",
        parse_all: rust::parse_all,
    },
];

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
    let Some(lang_def) = LANGUAGES.iter().find(|l| l.canonical == language) else {
        return FileResult { path: path.to_path_buf(), language, loc, entities: vec![], imports: vec![], dependencies: vec![], errors: vec![] };
    };
    let (entities, imports, errors) = (lang_def.parse_all)(source, &loc_map);
    FileResult { path: path.to_path_buf(), language, loc, entities, imports, dependencies: vec![], errors }
}

pub(crate) fn resolve_imports(
    language: &str,
    imports: &[String],
    project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    LANGUAGES
        .iter()
        .find(|l| l.canonical == language)
        .map(|l| match l.canonical {
            "C" => c::resolve_imports(imports, project_files),
            "C++" => cpp::resolve_imports(imports, project_files),
            "Rust" => rust::resolve_imports(imports, project_files),
            "QML" => qml::resolve_imports(imports, project_files),
            _ => vec![],
        })
        .unwrap_or_default()
}

pub(crate) fn verify_internal_deps(
    dependencies: &[Dependency],
    project_files: &std::collections::HashSet<std::path::PathBuf>,
) -> Vec<Dependency> {
    let stems: std::collections::HashSet<String> = project_files
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_owned()))
        .collect();
    dependencies
        .iter()
        .map(|d| {
            if matches!(d.kind, DependencyKind::Internal)
                && !stems.contains(d.name.as_str())
            {
                Dependency { name: d.name.clone(), kind: DependencyKind::External }
            } else {
                d.clone()
            }
        })
        .collect()
}

pub(super) fn parse_source(language: tree_sitter::Language, source: &str) -> Option<tree_sitter::Tree> {
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
        if import_kinds.contains(&child.kind()) && let Ok(text) = child.utf8_text(source.as_bytes()) {
            imports.push(text.trim().to_owned());
        }
    }
    imports
}

pub(super) fn collect_children<F>(
    node: Node<'_>,
    source: &str,
    loc_map: &LocMap,
    context: bool,
    depth: usize,
    f: F,
) -> Vec<Entity>
where
    F: Fn(Node<'_>, &str, &LocMap, bool, usize) -> Option<Entity>,
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
    fn detect_rust() {
        assert_eq!(detect(Path::new("foo.rs")), Some("Rust"));
    }

    #[test]
    fn detect_qml() {
        assert_eq!(detect(Path::new("foo.qml")), Some("QML"));
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect(Path::new("foo.py")), None);
        assert_eq!(detect(Path::new("foo")), None);
        assert_eq!(detect(Path::new("Makefile")), None);
    }

    #[test]
    fn detect_path_without_extension() {
        assert_eq!(detect(Path::new("Makefile")), None);
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
        let e = errors.iter().find(|e| e.kind == "error").expect("should have error kind");
        assert_eq!(e.node_kind, "ERROR");
        assert_eq!(e.start_line, 1);
        assert_eq!(e.end_line, 1);
    }

    #[test]
    fn detect_missing_node() {
        let src = "int x = 1\nint y = 2;\n";
        let tree = parse_source(tree_sitter_c::LANGUAGE.into(), src).unwrap();
        let errors = detect_syntax_errors(&tree);
        assert!(!errors.is_empty());
        let e = errors.iter().find(|e| e.kind == "missing").expect("should have missing kind");
        assert_eq!(e.start_line, 1);
        assert!(!e.node_kind.is_empty());
    }
}
