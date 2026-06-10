use super::common::{
    attach_docs_by_proximity, next_nonempty_line, visibility_from_explicit_public,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct ZigDocAdapter;

impl ZigDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("zig:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for ZigDocAdapter {
    fn language_id(&self) -> &'static str {
        "zig"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["zig"]
    }

    fn capabilities(&self) -> AdapterCapability {
        AdapterCapability {
            supports_functions: true,
            supports_types: true,
            supports_methods: false,
            supports_inline_docs: true,
        }
    }

    fn extract_symbols(&self, source: &str, path: &Path) -> Result<Vec<DocSymbol>, DocgenError> {
        static RE_FN: OnceLock<Regex> = OnceLock::new();
        static RE_CONST: OnceLock<Regex> = OnceLock::new();
        static RE_STRUCT: OnceLock<Regex> = OnceLock::new();
        static RE_ENUM: OnceLock<Regex> = OnceLock::new();
        let re_fn = RE_FN.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)")
                .expect("zig fn regex")
        });
        let re_const = RE_CONST.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=")
                .expect("zig const regex")
        });
        let re_struct = RE_STRUCT.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*struct")
                .expect("zig struct regex")
        });
        let re_enum = RE_ENUM.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*enum")
                .expect("zig enum regex")
        });

        let mut symbols = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if let Some(caps) = re_fn.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Function),
                    language: "zig".to_string(),
                    kind: DocSymbolKind::Function,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(format!("fn({})", args)),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_struct.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Struct),
                    language: "zig".to_string(),
                    kind: DocSymbolKind::Struct,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_enum.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Enum),
                    language: "zig".to_string(),
                    kind: DocSymbolKind::Enum,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_const.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Constant),
                    language: "zig".to_string(),
                    kind: DocSymbolKind::Constant,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
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
        let mut blocks = Vec::new();
        let lines: Vec<&str> = source.lines().collect();
        let mut idx = 0usize;

        while idx < lines.len() {
            let trimmed = lines[idx].trim();
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                let start = idx + 1;
                let mut content = Vec::new();
                let mut end = start;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if !(candidate.starts_with("///") || candidate.starts_with("//!")) {
                        break;
                    }
                    content.push(
                        candidate
                            .trim_start_matches("///")
                            .trim_start_matches("//!")
                            .trim()
                            .to_string(),
                    );
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
            idx += 1;
        }

        Ok(blocks)
    }

    fn attach_docs(&self, symbols: Vec<DocSymbol>, docs: Vec<DocCommentBlock>) -> Vec<DocSymbol> {
        attach_docs_by_proximity(symbols, docs)
    }
}
