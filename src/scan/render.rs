use crate::{
    lang,
    model::{DependencyKind, Entity, EntityType, FileResult, SyntaxError},
};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

// ── Tree output ───────────────────────────────────────────────────────────────

pub(super) fn print_tree(results: &[FileResult]) {
    let col = results
        .iter()
        .map(|r| header_width(r).max(entity_tree_width(&r.entities, 0)))
        .max()
        .unwrap_or(0)
        + 3;

    let mut max_loc = 1usize;
    let mut max_line = 1usize;
    for r in results {
        max_loc = max_loc.max(r.loc);
        collect_max_values(&r.entities, &mut max_loc, &mut max_line);
    }
    let lw = max_loc.to_string().len();
    let rw = max_line.to_string().len();

    let total_entities: usize = results.iter().map(|r| count_entities(&r.entities)).sum();

    for (i, result) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        let header = format!(
            "{} {}  [{}]",
            lang_icon(result.language),
            result.path.display(),
            result.language,
        );
        let n = count_entities(&result.entities);
        let deps_str = format_deps(result);
        let errors_str = format_errors(result);
        let suffix = match (deps_str.is_empty(), errors_str.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("  {errors_str}"),
            (false, true) => format!("  {deps_str}"),
            (false, false) => format!("  {deps_str}  {errors_str}"),
        };
        println!(
            "{}{}  {:>lw$} LOC  {}{}",
            header,
            " ".repeat(col - char_width(&header)),
            result.loc,
            plural(n, "entity", "entities"),
            suffix,
        );
        print_entities(&result.entities, "", col, lw, rw, &result.errors);
    }

    println!(
        "\n{}  {}",
        plural(results.len(), "file", "files"),
        plural(total_entities, "entity", "entities"),
    );
}

fn print_entities(entities: &[Entity], prefix: &str, col: usize, lw: usize, rw: usize, errors: &[SyntaxError]) {
    let count = entities.len();
    for (i, e) in entities.iter().enumerate() {
        let is_last = i + 1 == count;
        let connector = if is_last { "└─" } else { "├─" };
        let label = format!(
            "{}{} {} {}",
            prefix,
            connector,
            entity_icon(&e.entity_type),
            e.name,
        );
        let status = if entity_has_error(e, errors) { "⚠" } else { "✓" };
        println!(
            "{}{}  {:>lw$} LOC  [{:>rw$}-{:>rw$}]  {}",
            label,
            " ".repeat(col - char_width(&label)),
            e.loc,
            e.start_line,
            e.end_line,
            status,
        );
        if !e.children.is_empty() {
            let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
            print_entities(&e.children, &child_prefix, col, lw, rw, errors);
        }
    }
}

fn entity_has_error(entity: &Entity, errors: &[SyntaxError]) -> bool {
    errors.iter().any(|e| e.start_line <= entity.end_line && e.end_line >= entity.start_line)
}

fn header_width(result: &FileResult) -> usize {
    char_width(&format!(
        "{} {}  [{}]",
        lang_icon(result.language),
        result.path.display(),
        result.language,
    ))
}

fn entity_tree_width(entities: &[Entity], depth: usize) -> usize {
    entities
        .iter()
        .map(|e| {
            let w = depth * 3 + 5 + char_width(&e.name);
            let child_w = entity_tree_width(&e.children, depth + 1);
            w.max(child_w)
        })
        .max()
        .unwrap_or(0)
}

fn collect_max_values(entities: &[Entity], max_loc: &mut usize, max_line: &mut usize) {
    for e in entities {
        *max_loc = (*max_loc).max(e.loc);
        *max_line = (*max_line).max(e.end_line);
        collect_max_values(&e.children, max_loc, max_line);
    }
}

// ── JSON output ───────────────────────────────────────────────────────────────

pub(crate) fn results_to_json(results: &[FileResult], scan_path: &Path) -> serde_json::Value {
    let total_loc: usize = results.iter().map(|r| r.loc).sum();
    let total_entities: usize = results.iter().map(|r| count_entities(&r.entities)).sum();
    let total_errors: usize = results.iter().map(|r| r.errors.len()).sum();
    let files: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let path_str = r.path.display().to_string();
            let deps_json: Vec<serde_json::Value> = r
                .dependencies
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "name": d.name,
                        "kind": match d.kind {
                            DependencyKind::Internal => "internal",
                            DependencyKind::External => "external",
                        },
                    })
                })
                .collect();
            let errors_json: Vec<serde_json::Value> = r
                .errors
                .iter()
                .map(|e| serde_json::json!({
                    "kind": e.kind,
                    "node_kind": e.node_kind,
                    "start_line": e.start_line,
                    "end_line": e.end_line,
                }))
                .collect();
            serde_json::json!({
                "path": path_str,
                "language": r.language,
                "loc": r.loc,
                "entity_count": count_entities(&r.entities),
                "imports": r.imports,
                "dependencies": deps_json,
                "errors": errors_json,
                "entities": json_entities(&r.entities, &path_str, ""),
            })
        })
        .collect();
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "scan_path": scan_path.display().to_string(),
        "total_files": results.len(),
        "total_loc": total_loc,
        "total_entities": total_entities,
        "total_errors": total_errors,
        "files": files,
    })
}

pub(super) fn print_json(
    results: &[FileResult],
    scan_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&results_to_json(results, scan_path))?);
    Ok(())
}

pub(super) fn json_entities(
    entities: &[Entity],
    file_path: &str,
    parent_path: &str,
) -> Vec<serde_json::Value> {
    entities
        .iter()
        .map(|e| {
            let segment = entity_path_segment(&e.entity_type, &e.name);
            let entity_path = if parent_path.is_empty() {
                format!("{file_path} > {segment}")
            } else {
                format!("{parent_path} > {segment}")
            };
            serde_json::json!({
                "entity_path": entity_path,
                "name": e.name,
                "entity_type": entity_type_str(&e.entity_type),
                "loc": e.loc,
                "start_line": e.start_line,
                "end_line": e.end_line,
                "children": json_entities(&e.children, file_path, &entity_path),
            })
        })
        .collect()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub(super) fn entity_path_segment(et: &EntityType, name: &str) -> String {
    match et {
        EntityType::Impl => format!("impl {name}"),
        EntityType::Trait => format!("trait {name}"),
        EntityType::Namespace => format!("mod {name}"),
        _ => name.to_owned(),
    }
}

pub(super) fn entity_type_str(et: &EntityType) -> &'static str {
    match et {
        EntityType::Class => "Class",
        EntityType::Enum => "Enum",
        EntityType::Function => "Function",
        EntityType::Impl => "Impl",
        EntityType::Method => "Method",
        EntityType::Namespace => "Namespace",
        EntityType::Struct => "Struct",
        EntityType::Trait => "Trait",
    }
}

pub(super) fn count_entities(entities: &[Entity]) -> usize {
    entities
        .iter()
        .map(|e| 1 + count_entities(&e.children))
        .sum()
}

fn format_errors(r: &FileResult) -> String {
    let n = r.errors.len();
    if n == 0 { return String::new(); }
    plural(n, "error", "errors")
}

fn format_deps(r: &FileResult) -> String {
    let ext = r.dependencies.iter().filter(|d| matches!(d.kind, DependencyKind::External)).count();
    let int = r.dependencies.iter().filter(|d| matches!(d.kind, DependencyKind::Internal)).count();
    if ext == 0 && int == 0 {
        return String::new();
    }
    let parts: Vec<String> = match (ext, int) {
        (0, n) => vec![format!("{n} int deps")],
        (n, 0) => vec![format!("{n} ext deps")],
        (e, i) => vec![format!("{e} ext deps"), format!("{i} int deps")],
    };
    parts.join("  ")
}

fn plural(n: usize, singular: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {many}")
    }
}

fn char_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn lang_icon(language: &str) -> &'static str {
    lang::LANGUAGES
        .iter()
        .find(|l| l.canonical == language)
        .map(|l| l.icon)
        .unwrap_or("󰈔")
}

fn entity_icon(et: &EntityType) -> &'static str {
    match et {
        EntityType::Namespace => "󰅩",
        EntityType::Class => "󰌗",
        EntityType::Struct => "󰠱",
        EntityType::Enum => "󰒻",
        EntityType::Function => "󰊕",
        EntityType::Method => "󰊕",
        EntityType::Impl => "󰉺",
        EntityType::Trait => "󰜁",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_path_segment_prefixes_impl_trait_namespace() {
        assert_eq!(entity_path_segment(&EntityType::Impl, "Foo"), "impl Foo");
        assert_eq!(entity_path_segment(&EntityType::Trait, "Animal"), "trait Animal");
        assert_eq!(entity_path_segment(&EntityType::Namespace, "utils"), "mod utils");
    }

    #[test]
    fn entity_path_segment_passthrough_for_other_types() {
        assert_eq!(entity_path_segment(&EntityType::Struct, "Point"), "Point");
        assert_eq!(entity_path_segment(&EntityType::Class, "Vec"), "Vec");
        assert_eq!(entity_path_segment(&EntityType::Enum, "Color"), "Color");
        assert_eq!(entity_path_segment(&EntityType::Function, "run"), "run");
        assert_eq!(entity_path_segment(&EntityType::Method, "new"), "new");
    }

    #[test]
    fn entity_type_str_matches_variant_names() {
        assert_eq!(entity_type_str(&EntityType::Class), "Class");
        assert_eq!(entity_type_str(&EntityType::Enum), "Enum");
        assert_eq!(entity_type_str(&EntityType::Function), "Function");
        assert_eq!(entity_type_str(&EntityType::Impl), "Impl");
        assert_eq!(entity_type_str(&EntityType::Method), "Method");
        assert_eq!(entity_type_str(&EntityType::Namespace), "Namespace");
        assert_eq!(entity_type_str(&EntityType::Struct), "Struct");
        assert_eq!(entity_type_str(&EntityType::Trait), "Trait");
    }

    #[test]
    fn entity_has_error_overlapping_range() {
        use crate::model::SyntaxError;
        let entity = Entity::new("foo".into(), EntityType::Function, 5, 3, 10);
        let err = SyntaxError { kind: "error".into(), node_kind: "ERROR".into(), start_line: 5, end_line: 5 };
        assert!(entity_has_error(&entity, &[err]));
    }

    #[test]
    fn entity_has_error_non_overlapping_range() {
        use crate::model::SyntaxError;
        let entity = Entity::new("foo".into(), EntityType::Function, 5, 3, 10);
        let err = SyntaxError { kind: "error".into(), node_kind: "ERROR".into(), start_line: 15, end_line: 20 };
        assert!(!entity_has_error(&entity, &[err]));
    }

    #[test]
    fn entity_has_error_empty_errors() {
        let entity = Entity::new("foo".into(), EntityType::Function, 5, 3, 10);
        assert!(!entity_has_error(&entity, &[]));
    }

    #[test]
    fn format_errors_zero() {
        let r = FileResult {
            path: std::path::PathBuf::from("x.rs"),
            language: "Rust",
            loc: 1,
            entities: vec![],
            imports: vec![],
            dependencies: vec![],
            errors: vec![],
        };
        assert_eq!(format_errors(&r), "");
    }

    #[test]
    fn format_errors_singular_and_plural() {
        use crate::model::SyntaxError;
        let make = |n: usize| FileResult {
            path: std::path::PathBuf::from("x.rs"),
            language: "Rust",
            loc: 1,
            entities: vec![],
            imports: vec![],
            dependencies: vec![],
            errors: (0..n).map(|_| SyntaxError {
                kind: "error".into(), node_kind: "ERROR".into(), start_line: 1, end_line: 1,
            }).collect(),
        };
        assert_eq!(format_errors(&make(1)), "1 error");
        assert_eq!(format_errors(&make(2)), "2 errors");
    }
}
