use crate::{
    cache, config, lang,
    model::{Entity, EntityType, FileResult},
    walker,
};
use std::{fs, path::Path};

pub(crate) fn run(
    path: &Path,
    lang_filter: &[String],
    format: &str,
    match_pattern: &str,
    max_depth: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = build_scan_results(path, lang_filter, match_pattern, max_depth)?;
    match format {
        "json" => print_json(&results, path)?,
        _ => print_tree(&results),
    }
    Ok(())
}

pub(crate) fn build_scan_results(
    path: &Path,
    lang_filter: &[String],
    match_pattern: &str,
    max_depth: Option<usize>,
) -> Result<Vec<FileResult>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("'{}' does not exist", path.display()).into());
    }
    let cfg = config::load();
    let files = walker::walk(path, &cfg.scan.ignore_dirs, cfg.scan.follow_symlinks);

    let cache_path = std::path::Path::new(".code-seek/cache.json");
    let mut file_cache = cache::load(cache_path);
    let mut cache_dirty = false;

    let mut results: Vec<FileResult> = Vec::new();
    for file in &files {
        let language = match lang::detect(file) {
            Some(l) => l,
            None => continue,
        };
        if !lang_matches(language, lang_filter) {
            continue;
        }
        if let Ok(meta) = fs::metadata(file) {
            let size_mb = meta.len() as f64 / (1024.0 * 1024.0);
            if size_mb > cfg.scan.max_file_size_mb {
                crate::log::warn(&format!(
                    "skipping '{}' ({:.1} MB exceeds {:.0} MB limit)",
                    file.display(),
                    size_mb,
                    cfg.scan.max_file_size_mb,
                ));
                continue;
            }
        }
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                crate::log::warn(&format!("skipping '{}': {}", file.display(), e));
                continue;
            }
        };
        let key = file.display().to_string();
        let sha = cache::sha256(&source);
        if let Some(cached) = file_cache.get(&key, &sha) {
            results.push(cached);
            continue;
        }
        let file_result = lang::parse_file(file, &source, language);
        file_cache.insert(key, &file_result, sha);
        cache_dirty = true;
        results.push(file_result);
    }

    if cache_dirty {
        file_cache.save(cache_path);
    }

    let pattern = match_pattern.to_lowercase();
    if !pattern.is_empty() || max_depth.is_some() {
        for result in &mut results {
            let entities = std::mem::take(&mut result.entities);
            result.entities = filter_entities(entities, &pattern, max_depth, 0);
        }
    }

    Ok(results)
}

// ── Tree output ───────────────────────────────────────────────────────────────

fn print_tree(results: &[FileResult]) {
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
        println!(
            "{}{}  {:>lw$} LOC  {}",
            header,
            " ".repeat(col - char_width(&header)),
            result.loc,
            plural(n, "entity", "entities"),
        );
        print_entities(&result.entities, "", col, lw, rw);
    }

    println!(
        "\n{}  {}",
        plural(results.len(), "file", "files"),
        plural(total_entities, "entity", "entities"),
    );
}

fn print_entities(entities: &[Entity], prefix: &str, col: usize, lw: usize, rw: usize) {
    for (i, e) in entities.iter().enumerate() {
        let is_last = i == entities.len() - 1;
        let connector = if is_last { "└─" } else { "├─" };
        let label = format!(
            "{}{} {} {}",
            prefix,
            connector,
            entity_icon(&e.entity_type),
            e.name
        );
        println!(
            "{}{}  {:>lw$} LOC  [{:>rw$}-{:>rw$}]",
            label,
            " ".repeat(col - char_width(&label)),
            e.loc,
            e.start_line,
            e.end_line,
        );
        if !e.children.is_empty() {
            let child_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
            print_entities(&e.children, &child_prefix, col, lw, rw);
        }
    }
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
    let files: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            let path_str = r.path.display().to_string();
            serde_json::json!({
                "path": path_str,
                "language": r.language,
                "loc": r.loc,
                "entity_count": count_entities(&r.entities),
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
        "files": files,
    })
}

fn print_json(
    results: &[FileResult],
    scan_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&results_to_json(results, scan_path))?);
    Ok(())
}

fn json_entities(
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

fn entity_path_segment(et: &EntityType, name: &str) -> String {
    match et {
        EntityType::Impl => format!("impl {name}"),
        EntityType::Trait => format!("trait {name}"),
        EntityType::Namespace => format!("mod {name}"),
        _ => name.to_owned(),
    }
}

fn entity_type_str(et: &EntityType) -> &'static str {
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

// ── Shared helpers ────────────────────────────────────────────────────────────

fn filter_entities(
    entities: Vec<Entity>,
    pattern: &str,
    max_depth: Option<usize>,
    depth: usize,
) -> Vec<Entity> {
    if max_depth.is_some_and(|d| depth >= d) {
        return Vec::new();
    }
    entities
        .into_iter()
        .filter_map(|mut e| {
            e.children = filter_entities(e.children, pattern, max_depth, depth + 1);
            if pattern.is_empty() || e.name.to_lowercase().contains(pattern) || !e.children.is_empty() {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

fn count_entities(entities: &[Entity]) -> usize {
    entities
        .iter()
        .map(|e| 1 + count_entities(&e.children))
        .sum()
}

pub(crate) fn find_entity(results: &[FileResult], entity_path: &str) -> Option<serde_json::Value> {
    let mut parts = entity_path.splitn(2, " > ");
    let file_seg = parts.next()?;
    let rest = parts.next()?;

    let result = results.iter().find(|r| r.path.display().to_string() == file_seg)?;
    let file_path_str = result.path.display().to_string();

    let mut current: &[Entity] = &result.entities;
    let mut found: Option<&Entity> = None;

    for seg in rest.split(" > ") {
        found = current.iter().find(|e| entity_path_segment(&e.entity_type, &e.name) == seg);
        match found {
            Some(e) => current = &e.children,
            None => return None,
        }
    }

    found.map(|e| serde_json::json!({
        "entity_path": entity_path,
        "name": e.name,
        "entity_type": entity_type_str(&e.entity_type),
        "loc": e.loc,
        "start_line": e.start_line,
        "end_line": e.end_line,
        "children": json_entities(&e.children, &file_path_str, entity_path),
    }))
}

pub(crate) fn summarize(results: &[FileResult], scan_path: &Path) -> serde_json::Value {
    let total_loc: usize = results.iter().map(|r| r.loc).sum();
    let total_entities: usize = results.iter().map(|r| count_entities(&r.entities)).sum();

    let mut lang_map: std::collections::BTreeMap<&str, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for r in results {
        let e = lang_map.entry(r.language).or_default();
        e.0 += 1;
        e.1 += r.loc;
        e.2 += count_entities(&r.entities);
    }
    let by_language: Vec<serde_json::Value> = lang_map
        .iter()
        .map(|(lang, (files, loc, entities))| {
            serde_json::json!({"language": lang, "files": files, "loc": loc, "entities": entities})
        })
        .collect();

    let mut type_counts: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for r in results {
        count_by_type(&r.entities, &mut type_counts);
    }
    let by_entity_type: Vec<serde_json::Value> = type_counts
        .iter()
        .map(|(et, count)| serde_json::json!({"entity_type": et, "count": count}))
        .collect();

    serde_json::json!({
        "scan_path": scan_path.display().to_string(),
        "total_files": results.len(),
        "total_loc": total_loc,
        "total_entities": total_entities,
        "by_language": by_language,
        "by_entity_type": by_entity_type,
    })
}

pub(crate) fn locate(results: &[FileResult], name: &str) -> Vec<serde_json::Value> {
    let name_lower = name.to_lowercase();
    let mut matches = Vec::new();
    for result in results {
        let file_path = result.path.display().to_string();
        collect_matches(&result.entities, &name_lower, &file_path, &mut matches);
    }
    matches
}

fn collect_matches(
    entities: &[Entity],
    name_lower: &str,
    parent_path: &str,
    matches: &mut Vec<serde_json::Value>,
) {
    for e in entities {
        let segment = entity_path_segment(&e.entity_type, &e.name);
        let entity_path = format!("{parent_path} > {segment}");
        if e.name.to_lowercase() == name_lower {
            matches.push(serde_json::json!({
                "entity_path": entity_path,
                "name": e.name,
                "entity_type": entity_type_str(&e.entity_type),
                "loc": e.loc,
                "start_line": e.start_line,
                "end_line": e.end_line,
            }));
        }
        collect_matches(&e.children, name_lower, &entity_path, matches);
    }
}

fn count_by_type(
    entities: &[Entity],
    counts: &mut std::collections::BTreeMap<&'static str, usize>,
) {
    for e in entities {
        *counts.entry(entity_type_str(&e.entity_type)).or_default() += 1;
        count_by_type(&e.children, counts);
    }
}

fn lang_matches(language: &str, filter: &[String]) -> bool {
    if filter.is_empty() {
        return true;
    }
    lang::LANGUAGES
        .iter()
        .find(|l| l.canonical == language)
        .map(|l| {
            filter.iter().any(|f| {
                let lower = f.to_lowercase();
                l.aliases.iter().any(|&a| a == lower)
            })
        })
        .unwrap_or(false)
}

fn plural(n: usize, singular: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {many}")
    }
}

fn char_width(s: &str) -> usize {
    s.chars().count()
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
    use crate::model::EntityType;

    fn make_entity(name: &str, et: EntityType, children: Vec<Entity>) -> Entity {
        let mut e = Entity::new(name.to_owned(), et, 1, 1, 1);
        e.children = children;
        e
    }

    #[test]
    fn filter_depth_0_returns_empty() {
        let entities = vec![make_entity("foo", EntityType::Function, vec![])];
        assert!(filter_entities(entities, "", Some(0), 0).is_empty());
    }

    #[test]
    fn filter_depth_1_returns_roots_only() {
        let child = make_entity("bar", EntityType::Method, vec![]);
        let root = make_entity("Foo", EntityType::Impl, vec![child]);
        let result = filter_entities(vec![root], "", Some(1), 0);
        assert_eq!(result.len(), 1);
        assert!(result[0].children.is_empty());
    }

    #[test]
    fn filter_match_excludes_non_matching() {
        let a = make_entity("add", EntityType::Function, vec![]);
        let b = make_entity("sub", EntityType::Function, vec![]);
        let result = filter_entities(vec![a, b], "add", None, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "add");
    }

    #[test]
    fn filter_match_preserves_parent_with_matching_child() {
        let bar = make_entity("bar", EntityType::Method, vec![]);
        let baz = make_entity("baz", EntityType::Method, vec![]);
        let foo = make_entity("Foo", EntityType::Impl, vec![bar, baz]);
        let result = filter_entities(vec![foo], "bar", None, 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Foo");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[0].children[0].name, "bar");
    }

    #[test]
    fn filter_depth_wins_over_match() {
        // "bar" is at depth 1; with max_depth=1 it must be excluded even though it matches
        let bar = make_entity("bar", EntityType::Method, vec![]);
        let foo = make_entity("Foo", EntityType::Impl, vec![bar]);
        let result = filter_entities(vec![foo], "bar", Some(1), 0);
        // Foo is at depth 0 (included), but its children are pruned (depth 1 >= max_depth 1)
        // Foo itself doesn't match "bar", and has no children left → excluded
        assert!(result.is_empty());
    }

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

    fn make_file_result(path: &str, entities: Vec<Entity>) -> FileResult {
        FileResult { path: std::path::PathBuf::from(path), language: "Rust", loc: 10, entities }
    }

    #[test]
    fn find_entity_returns_match() {
        let child = make_entity("new", EntityType::Method, vec![]);
        let root = make_entity("Foo", EntityType::Impl, vec![child]);
        let results = vec![make_file_result("src/foo.rs", vec![root])];
        let found = find_entity(&results, "src/foo.rs > impl Foo > new").unwrap();
        assert_eq!(found["name"], "new");
        assert_eq!(found["entity_type"], "Method");
        assert_eq!(found["entity_path"], "src/foo.rs > impl Foo > new");
    }

    #[test]
    fn find_entity_returns_none_for_unknown_path() {
        let results = vec![make_file_result(
            "src/foo.rs",
            vec![make_entity("foo", EntityType::Function, vec![])],
        )];
        assert!(find_entity(&results, "src/bar.rs > foo").is_none());
    }

    #[test]
    fn find_entity_returns_none_for_unknown_entity() {
        let results = vec![make_file_result(
            "src/foo.rs",
            vec![make_entity("foo", EntityType::Function, vec![])],
        )];
        assert!(find_entity(&results, "src/foo.rs > bar").is_none());
    }

    #[test]
    fn summarize_counts_by_language() {
        let r1 = make_file_result("a.rs", vec![make_entity("f", EntityType::Function, vec![])]);
        let r2 = FileResult {
            path: std::path::PathBuf::from("b.rs"),
            language: "Rust",
            loc: 5,
            entities: vec![],
        };
        let summary = summarize(&[r1, r2], std::path::Path::new("."));
        let by_lang = summary["by_language"].as_array().unwrap();
        assert_eq!(by_lang.len(), 1);
        assert_eq!(by_lang[0]["language"], "Rust");
        assert_eq!(by_lang[0]["files"], 2);
        assert_eq!(by_lang[0]["entities"], 1);
    }

    #[test]
    fn locate_returns_matching_entities_across_files() {
        let r1 = make_file_result("src/a.rs", vec![make_entity("run", EntityType::Function, vec![])]);
        let r2 = make_file_result("src/b.rs", vec![make_entity("run", EntityType::Function, vec![])]);
        let matches = locate(&[r1, r2], "run");
        assert_eq!(matches.len(), 2);
        assert!(matches[0]["entity_path"].as_str().unwrap().starts_with("src/a.rs"));
        assert!(matches[1]["entity_path"].as_str().unwrap().starts_with("src/b.rs"));
    }

    #[test]
    fn locate_is_case_insensitive() {
        let results = vec![make_file_result("src/a.rs", vec![make_entity("Run", EntityType::Function, vec![])])];
        let matches = locate(&results, "run");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["name"], "Run");
    }

    #[test]
    fn locate_returns_empty_when_no_match() {
        let results = vec![make_file_result("src/a.rs", vec![make_entity("foo", EntityType::Function, vec![])])];
        assert!(locate(&results, "nonexistent").is_empty());
    }

    #[test]
    fn locate_finds_nested_entity() {
        let child = make_entity("new", EntityType::Method, vec![]);
        let root = make_entity("Foo", EntityType::Impl, vec![child]);
        let results = vec![make_file_result("src/foo.rs", vec![root])];
        let matches = locate(&results, "new");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["entity_path"], "src/foo.rs > impl Foo > new");
        assert_eq!(matches[0]["entity_type"], "Method");
    }

    #[test]
    fn summarize_counts_by_entity_type() {
        let entities = vec![
            make_entity("f", EntityType::Function, vec![]),
            make_entity("g", EntityType::Function, vec![]),
            make_entity("S", EntityType::Struct, vec![]),
        ];
        let results = vec![make_file_result("a.rs", entities)];
        let summary = summarize(&results, std::path::Path::new("."));
        let by_type = summary["by_entity_type"].as_array().unwrap();
        assert_eq!(by_type[0]["entity_type"], "Function");
        assert_eq!(by_type[0]["count"], 2);
        assert_eq!(by_type[1]["entity_type"], "Struct");
        assert_eq!(by_type[1]["count"], 1);
    }
}
