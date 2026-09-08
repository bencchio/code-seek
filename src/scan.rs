use crate::{
    config, lang,
    model::{Entity, EntityType, FileResult},
    walker,
};
use std::{fs, path::Path};

pub fn run(path: &Path, lang_filter: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("'{}' does not exist", path.display()).into());
    }
    let cfg = config::load();
    let files = walker::walk(path, &cfg.scan.ignore_dirs, cfg.scan.follow_symlinks);

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
                eprintln!(
                    "warning: skipping '{}' ({:.1} MB exceeds {:.0} MB limit)",
                    file.display(),
                    size_mb,
                    cfg.scan.max_file_size_mb,
                );
                continue;
            }
        }
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warning: skipping '{}': {}", file.display(), e);
                continue;
            }
        };
        results.push(lang::parse_file(file, &source, language));
    }

    let col = results
        .iter()
        .map(|r| header_width(r).max(entity_tree_width(&r.entities, 0)))
        .max()
        .unwrap_or(0)
        + 3;

    let mut max_loc = 1usize;
    let mut max_line = 1usize;
    for r in &results {
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
    Ok(())
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

fn count_entities(entities: &[Entity]) -> usize {
    entities
        .iter()
        .map(|e| 1 + count_entities(&e.children))
        .sum()
}

fn lang_matches(language: &str, filter: &[String]) -> bool {
    if filter.is_empty() {
        return true;
    }
    filter.iter().any(|f| match f.to_lowercase().as_str() {
        "c" => language == "C",
        "cpp" | "c++" => language == "C++",
        "qml" => language == "QML",
        "rust" => language == "Rust",
        _ => false,
    })
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
    match language {
        "C" => "󰙱",
        "C++" => "󰙲",
        "QML" => "󰈚",
        "Rust" => "󱘗",
        _ => "󰈔",
    }
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
