pub(crate) mod filter;
pub(crate) mod render;

pub(crate) use filter::{find_entity, locate, summarize};
pub(crate) use render::results_to_json;

use crate::{cache, config, gitignore, lang, model::FileResult, walker};
use std::collections::HashSet;
use std::{fs, path::Path};

pub(crate) fn run(
    path: &Path,
    lang_filter: &[String],
    format: &str,
    match_pattern: &str,
    max_depth: Option<usize>,
    extra_ignore: &[String],
    info: &str,
    respect_gitignore: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = build_scan_results(
        path,
        lang_filter,
        match_pattern,
        max_depth,
        extra_ignore,
        info,
        respect_gitignore,
    )?;
    match format {
        "json" => render::print_json(&results, path)?,
        _ => render::print_tree(&results),
    }
    Ok(())
}

pub(crate) fn build_scan_results(
    path: &Path,
    lang_filter: &[String],
    match_pattern: &str,
    max_depth: Option<usize>,
    extra_ignore: &[String],
    info: &str,
    respect_gitignore: bool,
) -> Result<Vec<FileResult>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("'{}' does not exist", path.display()).into());
    }
    for lang in lang_filter {
        let lower = lang.to_lowercase();
        if !lang::LANGUAGES
            .iter()
            .any(|l| l.aliases.contains(&lower.as_str()))
        {
            return Err(format!("unrecognized language: '{lang}'").into());
        }
    }
    let cfg = config::load();
    let all_ignore: Vec<String> = cfg
        .scan
        .ignore_dirs
        .iter()
        .cloned()
        .chain(extra_ignore.iter().cloned())
        .collect();
    let allowed = respect_gitignore
        .then(|| gitignore::allowed(path))
        .flatten();
    let files = walker::walk(
        path,
        &all_ignore,
        cfg.scan.follow_symlinks,
        allowed.as_ref(),
    );
    let slot = crate::state::slot_dir();
    let cache_path = slot.as_ref().map(|s| s.join("cache.json"));
    let mut file_cache = cache_path
        .as_deref()
        .map(cache::load)
        .unwrap_or_else(cache::empty);
    let mut cache_dirty = false;

    let mut results: Vec<FileResult> = files
        .iter()
        .filter_map(|f| scan_one(f, &cfg, lang_filter, &mut file_cache, &mut cache_dirty))
        .collect();

    if cache_dirty {
        if let Some(path) = cache_path.as_deref() {
            file_cache.save(path);
        }
    }
    resolve_dependencies(&mut results);
    apply_filters(&mut results, match_pattern, max_depth, info);
    Ok(results)
}

fn scan_one(
    file: &Path,
    cfg: &config::Config,
    lang_filter: &[String],
    cache: &mut cache::Cache,
    cache_dirty: &mut bool,
) -> Option<FileResult> {
    let language = lang::detect(file)?;
    if !lang_matches(language, lang_filter) {
        return None;
    }
    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            crate::log::warn(&format!("skipping '{}': {}", file.display(), e));
            return None;
        }
    };
    let size_mb = source.len() as f64 / (1024.0 * 1024.0);
    if size_mb > cfg.scan.max_file_size_mb {
        crate::log::warn(&format!(
            "skipping '{}' ({:.1} MB exceeds {:.0} MB limit)",
            file.display(),
            size_mb,
            cfg.scan.max_file_size_mb,
        ));
        return None;
    }
    let key = file.display().to_string();
    let sha = cache::sha256(&source);
    if let Some(cached) = cache.get(&key, &sha) {
        return Some(cached);
    }
    let result = lang::parse_file(file, &source, language);
    cache.insert(key, &result, sha);
    *cache_dirty = true;
    Some(result)
}

fn apply_filters(results: &mut [FileResult], pattern: &str, max_depth: Option<usize>, info: &str) {
    let pattern = pattern.to_lowercase();
    for result in results.iter_mut() {
        let entities = std::mem::take(&mut result.entities);
        let entities = filter::filter_entities(entities, &pattern, max_depth, 0);
        result.entities = filter::filter_by_info(entities, info);
    }
}

fn resolve_dependencies(results: &mut [FileResult]) {
    let project_files: HashSet<std::path::PathBuf> =
        results.iter().map(|r| r.path.clone()).collect();
    for r in results.iter_mut() {
        r.dependencies = lang::resolve_imports(r.language, &r.imports, &project_files);
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
