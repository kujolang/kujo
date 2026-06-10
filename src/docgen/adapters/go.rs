use super::common::{
    attach_docs_by_proximity, next_nonempty_line, visibility_from_leading_uppercase,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct GoDocAdapter;

impl GoDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("go:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for GoDocAdapter {
    fn language_id(&self) -> &'static str {
        "go"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["go"]
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
        static RE_TYPE: OnceLock<Regex> = OnceLock::new();
        static RE_FUNC: OnceLock<Regex> = OnceLock::new();
        static RE_METHOD: OnceLock<Regex> = OnceLock::new();
        let re_type = RE_TYPE.get_or_init(|| {
            Regex::new(r"^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+(struct|interface)")
                .expect("go type regex")
        });
        let re_func = RE_FUNC.get_or_init(|| {
            Regex::new(r"^\s*func\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)").expect("go func regex")
        });
        let re_method = RE_METHOD.get_or_init(|| {
            Regex::new(r"^\s*func\s*\(([^)]*)\)\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)")
                .expect("go method regex")
        });

        let mut symbols = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if let Some(caps) = re_type.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let type_kind = caps.get(2).map(|m| m.as_str()).unwrap_or("struct");
                let kind = if type_kind == "interface" {
                    DocSymbolKind::Interface
                } else {
                    DocSymbolKind::Struct
                };
                let visibility = visibility_from_leading_uppercase(&name);
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &kind),
                    language: "go".to_string(),
                    kind,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_method.captures(trimmed) {
                let recv = caps.get(1).map(|m| m.as_str()).unwrap_or("recv").to_string();
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let qualified = format!("{}::{}", recv, name);
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &qualified, &DocSymbolKind::Method),
                    language: "go".to_string(),
                    kind: DocSymbolKind::Method,
                    name,
                    qualified_name: qualified,
                    signature: Some(format!("method({})", args)),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: Some(recv),
                });
            } else if let Some(caps) = re_func.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Function),
                    language: "go".to_string(),
                    kind: DocSymbolKind::Function,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(format!("func {}({})", name, args)),
                    visibility: visibility_from_leading_uppercase(&name),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
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
            let line_no = idx + 1;
            let trimmed = lines[idx].trim();
            if trimmed.starts_with("//") {
                let start = line_no;
                let mut content = Vec::new();
                let mut end = line_no;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if !candidate.starts_with("//") {
                        break;
                    }
                    content.push(candidate.trim_start_matches("//").trim().to_string());
                    end = idx + 1;
                    idx += 1;
                }
                blocks.push(DocCommentBlock {
                    start_line: start,
                    end_line: end,
                    target_line_hint: next_nonempty_line(source, end),
                    lines: content,
                });
                continue;
            }
            if trimmed.starts_with("/*") {
                let start = line_no;
                let mut content = Vec::new();
                let mut end = line_no;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    let cleaned = candidate
                        .trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim_start_matches('*')
                        .trim()
                        .to_string();
                    if !cleaned.is_empty() {
                        content.push(cleaned);
                    }
                    if candidate.contains("*/") {
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
