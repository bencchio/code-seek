use std::path::PathBuf;

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
}
