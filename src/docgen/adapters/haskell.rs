use super::common::{attach_docs_by_proximity, next_nonempty_line};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct HaskellDocAdapter;

impl HaskellDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("haskell:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for HaskellDocAdapter {
    fn language_id(&self) -> &'static str {
        "haskell"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["hs", "lhs"]
    }

    fn capabilities(&self) -> AdapterCapability {
        AdapterCapability {
            supports_functions: true,
            supports_types: true,
            supports_methods: true,
            supports_inline_docs: true,
        }
    }

    fn extract_symbols(&self, source: &str, path: &Path) -> Result<Vec<DocSymbol>, DocgenError> {
        static RE_MODULE: OnceLock<Regex> = OnceLock::new();
        static RE_DATA: OnceLock<Regex> = OnceLock::new();
        static RE_TYPECLASS: OnceLock<Regex> = OnceLock::new();
        static RE_FUNCTION: OnceLock<Regex> = OnceLock::new();
        let re_module = RE_MODULE.get_or_init(|| {
            Regex::new(r"^\s*module\s+([A-Za-z0-9_.']+)").expect("hs module regex")
        });
        let re_data = RE_DATA.get_or_init(|| {
            Regex::new(r"^\s*(data|newtype)\s+([A-Za-z_][A-Za-z0-9_']*)").expect("hs data regex")
        });
        let re_typeclass = RE_TYPECLASS.get_or_init(|| {
            Regex::new(r"^\s*class\s+([A-Za-z_][A-Za-z0-9_']*)").expect("hs class regex")
        });
        let re_function = RE_FUNCTION.get_or_init(|| {
            Regex::new(r"^\s*([a-z_][A-Za-z0-9_']*)\s*::?").expect("hs function regex")
        });

        let mut symbols = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if let Some(caps) = re_module.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("Main").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Module),
                    language: "haskell".to_string(),
                    kind: DocSymbolKind::Module,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_data.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("Type").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Struct),
                    language: "haskell".to_string(),
                    kind: DocSymbolKind::Struct,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_typeclass.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("TypeClass").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Trait),
                    language: "haskell".to_string(),
                    kind: DocSymbolKind::Trait,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_function.captures(trimmed) {
                if !trimmed.starts_with("--") {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("fn").to_string();
                    symbols.push(DocSymbol {
                        id: Self::id(path, line_no, &name, &DocSymbolKind::Function),
                        language: "haskell".to_string(),
                        kind: DocSymbolKind::Function,
                        name: name.clone(),
                        qualified_name: name,
                        signature: Some(trimmed.to_string()),
                        visibility: DocVisibility::Public,
                        source_path: path.to_path_buf(),
                        line: line_no,
                        docs: DocComment::default(),
                        examples: Vec::new(),
                        gaps: Vec::new(),
                        parent: None,
                    });
                }
            }
        }

        Ok(symbols)
    }

    fn extract_inline_docs(
        &self,
        source: &str,
        _path: &Path,
    ) -> Result<Vec<DocCommentBlock>, DocgenError> {
        let lines: Vec<&str> = source.lines().collect();
        let mut blocks = Vec::new();
        let mut idx = 0usize;

        while idx < lines.len() {
            let trimmed = lines[idx].trim();
            let line_no = idx + 1;
            if trimmed.starts_with("-- |") {
                blocks.push(DocCommentBlock {
                    start_line: line_no,
                    end_line: line_no,
                    target_line_hint: next_nonempty_line(source, line_no),
                    lines: vec![trimmed.trim_start_matches("-- |").trim().to_string()],
                });
            } else if trimmed.starts_with("{-|") {
                let start = line_no;
                let mut content = Vec::new();
                let mut end = line_no;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    let cleaned = candidate
                        .trim_start_matches("{-|")
                        .trim_end_matches("-}")
                        .trim_start_matches('*')
                        .trim()
                        .to_string();
                    if !cleaned.is_empty() {
                        content.push(cleaned);
                    }
                    if candidate.contains("-}") {
                        end = idx + 1;
                        break;
                    }
                    idx += 1;
                }
                blocks.push(DocCommentBlock {
                    start_line: start,
                    end_line: end,
                    target_line_hint: next_nonempty_line(source, end),
                    lines: content,
                });
            }
            idx += 1;
        }

        Ok(blocks)
    }

    fn attach_docs(&self, symbols: Vec<DocSymbol>, docs: Vec<DocCommentBlock>) -> Vec<DocSymbol> {
        attach_docs_by_proximity(symbols, docs)
    }
}
