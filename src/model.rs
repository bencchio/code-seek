use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum EntityType {
    Class,
    Enum,
    Function,
    Impl,
    Method,
    Namespace,
    Struct,
    Trait,
}

#[derive(Debug)]
pub struct Entity {
    pub name: String,
    pub entity_type: EntityType,
    pub loc: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub children: Vec<Entity>,
}

#[derive(Debug)]
pub struct FileResult {
    pub path: PathBuf,
    pub language: &'static str,
    pub loc: usize,
    pub entities: Vec<Entity>,
}
