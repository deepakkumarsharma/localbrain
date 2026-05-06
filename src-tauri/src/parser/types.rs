use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedFile {
    pub path: String,
    pub language: SourceLanguage,
    pub symbols: Vec<CodeSymbol>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourceLanguage {
    JavaScript,
    TypeScript,
    Tsx,
    Jsx,
    Rust,
    Go,
    Python,
    Java,
    Kotlin,
    Swift,
    Ruby,
    Php,
    C,
    Cpp,
    CSharp,
    Shell,
    Sql,
    Json,
    Yaml,
    Toml,
    Ini,
    Xml,
    Css,
    Scss,
    Less,
    Vue,
    Svelte,
    Astro,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub parent: Option<String>,
    pub source: Option<String>,
    pub range: SourceRange,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Component,
    Class,
    Method,
    Object,
    Enum,
    Interface,
    TypeAlias,
    Import,
    Export,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}
