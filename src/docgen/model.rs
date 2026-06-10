use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type DocSymbolId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocProject {
    pub name: Option<String>,
    pub root: PathBuf,
    pub languages: Vec<String>,
    pub modules: Vec<DocModule>,
    pub symbols: Vec<DocSymbol>,
    pub gaps: Vec<DocGap>,
    pub diagnostics: Vec<DocDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocModule {
    pub name: String,
    pub language: String,
    pub path: PathBuf,
    pub symbols: Vec<DocSymbolId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocSymbol {
    pub id: DocSymbolId,
    pub language: String,
    pub kind: DocSymbolKind,
    pub name: String,
    pub qualified_name: String,
    pub signature: Option<String>,
    pub visibility: DocVisibility,
    pub source_path: PathBuf,
    pub line: usize,
    pub docs: DocComment,
    pub examples: Vec<DocExample>,
    pub gaps: Vec<DocGapKind>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocSymbolKind {
    Module,
    Function,
    Method,
    Class,
    Struct,
    Enum,
    EnumVariant,
    Interface,
    Trait,
    TypeAlias,
    Constant,
    Variable,
    Property,
    Builtin,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocVisibility {
    Public,
    Private,
    Protected,
    Internal,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocComment {
    pub lines: Vec<String>,
    pub summary: Option<String>,
    pub placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocExample {
    pub language: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocCommentBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub target_line_hint: Option<usize>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocGap {
    pub id: String,
    pub language: String,
    pub symbol_id: DocSymbolId,
    pub symbol_name: String,
    pub symbol_kind: DocSymbolKind,
    pub signature: Option<String>,
    pub source_path: PathBuf,
    pub line: usize,
    pub missing_sections: Vec<DocGapKind>,
    pub existing_docs: Vec<String>,
    pub bounded_source_context: Vec<String>,
    pub known_call_sites: Vec<String>,
    pub suggested_ai_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocGapKind {
    MissingSummary,
    MissingExamples,
    MissingDocs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocDiagnostic {
    pub severity: DocDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocDiagnosticSeverity {
    Info,
    Warning,
    Error,
}
