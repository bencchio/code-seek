use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SyntaxError {
    pub(crate) kind: String,
    pub(crate) node_kind: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum DependencyKind {
    Internal,
    External,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Dependency {
    pub(crate) name: String,
    pub(crate) kind: DependencyKind,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum EntityType {
    Class,
    Enum,
    Function,
    Impl,
    Method,
    Namespace,
    Struct,
    Trait,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Entity {
    pub(crate) name: String,
    pub(crate) entity_type: EntityType,
    pub(crate) loc: usize,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) children: Vec<Entity>,
}

impl Entity {
    pub(crate) fn new(
        name: String,
        entity_type: EntityType,
        loc: usize,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self { name, entity_type, loc, start_line, end_line, children: Vec::new() }
    }
}

#[derive(Debug)]
pub(crate) struct FileResult {
    pub(crate) path: PathBuf,
    pub(crate) language: &'static str,
    pub(crate) loc: usize,
    pub(crate) entities: Vec<Entity>,
    pub(crate) imports: Vec<String>,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) errors: Vec<SyntaxError>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_error_fields() {
        let e = SyntaxError {
            kind: "error".into(),
            node_kind: "ERROR".into(),
            start_line: 1,
            end_line: 2,
        };
        assert_eq!(e.kind, "error");
        assert_eq!(e.node_kind, "ERROR");
        assert_eq!(e.start_line, 1);
        assert_eq!(e.end_line, 2);
    }

    #[test]
    fn file_result_fields() {
        let r = FileResult {
            path: std::path::PathBuf::from("src/main.rs"),
            language: "Rust",
            loc: 42,
            entities: vec![],
            imports: vec![],
            dependencies: vec![],
            errors: vec![],
        };
        assert_eq!(r.path, std::path::PathBuf::from("src/main.rs"));
        assert_eq!(r.language, "Rust");
        assert_eq!(r.loc, 42);
        assert!(r.entities.is_empty());
        assert!(r.errors.is_empty());
    }
}
