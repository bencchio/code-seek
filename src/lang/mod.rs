use std::path::Path;

use tree_sitter::{Language, Node, Parser};

use crate::model::{Entity, FileResult};

pub(crate) mod c;
pub(crate) mod cpp;
pub(crate) mod qml;
pub(crate) mod rust;

pub(crate) struct LangDef {
    pub(crate) canonical: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) extensions: &'static [&'static str],
    pub(crate) icon: &'static str,
    parser: fn(&str, &LocMap) -> Vec<Entity>,
}

pub(crate) static LANGUAGES: &[LangDef] = &[
    LangDef {
        canonical: "C",
        aliases: &["c"],
        extensions: &["c", "h"],
        icon: "󰙱",
        parser: parse_c,
    },
    LangDef {
        canonical: "C++",
        aliases: &["cpp", "c++"],
        extensions: &["cpp", "cc", "cxx", "hpp", "hxx", "h++"],
        icon: "󰙲",
        parser: parse_cpp,
    },
    LangDef {
        canonical: "QML",
        aliases: &["qml"],
        extensions: &["qml"],
        icon: "󰈚",
        parser: parse_qml,
    },
    LangDef {
        canonical: "Rust",
        aliases: &["rust"],
        extensions: &["rs"],
        icon: "󱘗",
        parser: parse_rust,
    },
];

fn parse_c(src: &str, loc_map: &LocMap) -> Vec<Entity> { c::parse_impl(src, loc_map) }
fn parse_cpp(src: &str, loc_map: &LocMap) -> Vec<Entity> { cpp::parse_impl(src, loc_map) }
fn parse_qml(src: &str, loc_map: &LocMap) -> Vec<Entity> { qml::parse_impl(src, loc_map) }
fn parse_rust(src: &str, loc_map: &LocMap) -> Vec<Entity> { rust::parse_impl(src, loc_map) }

pub(crate) struct LocMap {
    prefix: Vec<usize>,
}

impl LocMap {
    pub(crate) fn build(source: &str) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let n = lines.len();
        let mut prefix = vec![0usize; n + 1];
        let mut in_block = false;
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            let is_loc = if in_block {
                if t.contains("*/") {
                    in_block = false;
                }
                false
            } else if t.is_empty() || t.starts_with("//") || t.starts_with('*') {
                false
            } else if t.starts_with("/*") {
                if !t.contains("*/") {
                    in_block = true;
                }
                false
            } else {
                if let Some(pos) = t.find("/*")
                    && !t[pos..].contains("*/")
                {
                    in_block = true;
                }
                true
            };
            prefix[i + 1] = prefix[i] + is_loc as usize;
        }
        Self { prefix }
    }

    pub(crate) fn count(&self, start_line: usize, end_line: usize) -> usize {
        let n = self.prefix.len().saturating_sub(1);
        let e = end_line.min(n);
        let s = start_line.saturating_sub(1);
        self.prefix[e].saturating_sub(self.prefix[s])
    }

    pub(crate) fn total(&self) -> usize {
        *self.prefix.last().unwrap_or(&0)
    }
}

pub(crate) fn parse_source(language: Language, source: &str) -> Option<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language).expect("failed to set tree-sitter language");
    parser.parse(source.as_bytes(), None)
}

pub(crate) fn collect_children<F>(
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
        return FileResult { path: path.to_path_buf(), language, loc: 0, entities: Vec::new() };
    };
    let entities = (lang_def.parser)(source, &loc_map);
    FileResult { path: path.to_path_buf(), language, loc, entities }
}

pub(crate) fn node_lines(node: Node<'_>) -> (usize, usize) {
    (node.start_position().row + 1, node.end_position().row + 1)
}

pub(crate) fn declarator_name(node: Node<'_>, src: &[u8]) -> Option<String> {
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
    fn loc_map_counts_code_lines() {
        let src = "int x = 1;\n\n// comment\n/* block */\n* continuation\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.total(), 2);
    }

    #[test]
    fn loc_map_skips_multiline_block_comment() {
        let src = "int x = 1;\n/*\n   inside comment\n   no star prefix\n*/\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.total(), 2);
    }

    #[test]
    fn loc_map_range_is_inclusive() {
        let src = "line1\nline2\nline3\nline4\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(2, 3), 2);
    }

    #[test]
    fn loc_map_single_line() {
        let src = "int x = 1;\n\nint y = 2;\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(1, 1), 1);
        assert_eq!(map.count(2, 2), 0);
        assert_eq!(map.count(3, 3), 1);
    }

    #[test]
    fn loc_map_total_matches_full_range() {
        let src = "a\nb\n\nc\n";
        let map = LocMap::build(src);
        assert_eq!(map.count(1, 4), map.total());
    }
}
