use crate::{
    model::{Entity, EntityType, FileResult},
    scan::render,
};

pub(super) fn filter_by_info(entities: Vec<Entity>, info: &str) -> Vec<Entity> {
    match info {
        "no-tests" => strip_test_modules(entities),
        "tests-only" => keep_test_modules_only(entities),
        _ => entities,
    }
}

fn strip_test_modules(entities: Vec<Entity>) -> Vec<Entity> {
    entities
        .into_iter()
        .filter_map(|mut e| {
            if e.entity_type == EntityType::Namespace && e.name == "tests" {
                return None;
            }
            e.children = strip_test_modules(e.children);
            Some(e)
        })
        .collect()
}

fn keep_test_modules_only(entities: Vec<Entity>) -> Vec<Entity> {
    keep_test_mods_inner(entities, false)
}

fn keep_test_mods_inner(entities: Vec<Entity>, inside: bool) -> Vec<Entity> {
    entities
        .into_iter()
        .filter_map(|mut e| {
            let is_test_mod = e.entity_type == EntityType::Namespace && e.name == "tests";
            e.children = keep_test_mods_inner(e.children, inside || is_test_mod);
            if is_test_mod || inside || !e.children.is_empty() {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn filter_entities(
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

pub(crate) fn find_entity(results: &[FileResult], entity_path: &str) -> Option<serde_json::Value> {
    let (file_seg, rest) = entity_path.split_once(" > ")?;

    let result = results.iter().find(|r| r.path.display().to_string() == file_seg)?;
    let file_path_str = result.path.display().to_string();

    let mut current: &[Entity] = &result.entities;
    let mut found: Option<&Entity> = None;

    for seg in rest.split(" > ") {
        found = current.iter().find(|e| render::entity_path_segment(&e.entity_type, &e.name) == seg);
        match found {
            Some(e) => current = &e.children,
            None => return None,
        }
    }

    found.map(|e| serde_json::json!({
        "entity_path": entity_path,
        "name": e.name,
        "entity_type": render::entity_type_str(&e.entity_type),
        "loc": e.loc,
        "start_line": e.start_line,
        "end_line": e.end_line,
        "children": render::json_entities(&e.children, &file_path_str, entity_path),
    }))
}

pub(crate) fn summarize(results: &[FileResult], scan_path: &std::path::Path) -> serde_json::Value {
    let total_loc: usize = results.iter().map(|r| r.loc).sum();
    let total_entities: usize = results.iter().map(|r| render::count_entities(&r.entities)).sum();

    let mut lang_map: std::collections::BTreeMap<&str, (usize, usize, usize)> =
        std::collections::BTreeMap::new();
    for r in results {
        let e = lang_map.entry(r.language).or_default();
        e.0 += 1;
        e.1 += r.loc;
        e.2 += render::count_entities(&r.entities);
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
        let segment = render::entity_path_segment(&e.entity_type, &e.name);
        let entity_path = format!("{parent_path} > {segment}");
        if e.name.to_lowercase() == name_lower {
            matches.push(serde_json::json!({
                "entity_path": entity_path,
                "name": e.name,
                "entity_type": render::entity_type_str(&e.entity_type),
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
        *counts.entry(render::entity_type_str(&e.entity_type)).or_default() += 1;
        count_by_type(&e.children, counts);
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

    fn make_file_result(path: &str, entities: Vec<Entity>) -> FileResult {
        FileResult { path: std::path::PathBuf::from(path), language: "Rust", loc: 10, entities, imports: vec![], dependencies: vec![], errors: vec![] }
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
        let bar = make_entity("bar", EntityType::Method, vec![]);
        let foo = make_entity("Foo", EntityType::Impl, vec![bar]);
        let result = filter_entities(vec![foo], "bar", Some(1), 0);
        assert!(result.is_empty());
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
            imports: vec![],
            dependencies: vec![],
            errors: vec![],
        };
        let summary = summarize(&[r1, r2], std::path::Path::new("."));
        let by_lang = summary["by_language"].as_array().unwrap();
        assert_eq!(by_lang.len(), 1);
        assert_eq!(by_lang[0]["language"], "Rust");
        assert_eq!(by_lang[0]["files"], 2);
        assert_eq!(by_lang[0]["entities"], 1);
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

    #[test]
    fn filter_by_info_all_preserves_all() {
        let e = make_entity("foo", EntityType::Function, vec![]);
        let result = filter_by_info(vec![e.clone()], "all");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "foo");
    }

    #[test]
    fn filter_by_info_no_tests_removes_test_mod() {
        let child = make_entity("helper", EntityType::Function, vec![]);
        let test_mod = make_entity("tests", EntityType::Namespace, vec![child]);
        let func = make_entity("main", EntityType::Function, vec![]);
        let result = filter_by_info(vec![test_mod, func], "no-tests");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "main");
    }

    #[test]
    fn filter_by_info_tests_only_keeps_test_mod() {
        let child = make_entity("helper", EntityType::Function, vec![]);
        let test_mod = make_entity("tests", EntityType::Namespace, vec![child]);
        let func = make_entity("main", EntityType::Function, vec![]);
        let result = filter_by_info(vec![test_mod, func], "tests-only");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "tests");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[0].children[0].name, "helper");
    }

    #[test]
    fn filter_by_info_tests_only_drops_other_entities() {
        let func = make_entity("main", EntityType::Function, vec![]);
        let result = filter_by_info(vec![func], "tests-only");
        assert!(result.is_empty());
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
}
