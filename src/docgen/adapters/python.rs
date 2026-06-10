use super::common::{
    attach_docs_by_proximity, next_nonempty_line, visibility_from_leading_underscore,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct PythonDocAdapter;

impl PythonDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("python:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for PythonDocAdapter {
    fn language_id(&self) -> &'static str {
        "python"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["py"]
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
        static RE_CLASS: OnceLock<Regex> = OnceLock::new();
        static RE_DEF: OnceLock<Regex> = OnceLock::new();
        let re_class = RE_CLASS.get_or_init(|| {
            Regex::new(r"^(\s*)class\s+([A-Za-z_][A-Za-z0-9_]*)").expect("python class regex")
        });
        let re_def = RE_DEF.get_or_init(|| {
            Regex::new(r"^(\s*)def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)")
                .expect("python def regex")
        });

        let mut symbols = Vec::new();
        let mut class_stack: Vec<(String, usize)> = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let indent = line.chars().take_while(|ch| ch.is_ascii_whitespace()).count();

            while class_stack.last().is_some_and(|(_, level)| indent <= *level) {
                class_stack.pop();
            }

            if let Some(caps) = re_class.captures(line) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Class),
                    language: "python".to_string(),
                    kind: DocSymbolKind::Class,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(line.trim().to_string()),
                    visibility: visibility_from_leading_underscore(&name),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                class_stack.push((name, indent));
            } else if let Some(caps) = re_def.captures(line) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let parent = class_stack.last().map(|entry| entry.0.clone());
                let kind =
                    if parent.is_some() { DocSymbolKind::Method } else { DocSymbolKind::Function };
                let qualified_name = if let Some(class_name) = &parent {
                    format!("{}.{}", class_name, name)
                } else {
                    name.clone()
                };
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &qualified_name, &kind),
                    language: "python".to_string(),
                    kind,
                    name: name.clone(),
                    qualified_name,
                    signature: Some(format!("def {}({})", name, args)),
                    visibility: visibility_from_leading_underscore(&name),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent,
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

        for (idx, line) in lines.iter().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if trimmed.starts_with('#') {
                let mut comments = Vec::new();
                let start = line_no;
                let mut end = line_no;
                let mut cursor = idx;
                while cursor < lines.len() {
                    let candidate = lines[cursor].trim();
                    if !candidate.starts_with('#') {
                        break;
                    }
                    comments.push(candidate.trim_start_matches('#').trim().to_string());
                    end = cursor + 1;
                    cursor += 1;
                }
                if !comments.is_empty() {
                    blocks.push(DocCommentBlock {
                        start_line: start,
                        end_line: end,
                        target_line_hint: next_nonempty_line(source, end),
                        lines: comments,
                    });
                }
            }

            if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                let delimiter = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
                let start = line_no;
                let mut content = Vec::new();
                let mut cursor = idx;
                let end = loop {
                    let candidate = lines[cursor].trim();
                    let cleaned = candidate.trim_matches('"').trim_matches('\'').trim().to_string();
                    if !cleaned.is_empty() {
                        content.push(cleaned);
                    }
                    if cursor > idx && candidate.ends_with(delimiter) {
                        break cursor + 1;
                    }
                    if cursor == idx && candidate.matches(delimiter).count() >= 2 {
                        break cursor + 1;
                    }
                    cursor += 1;
                    if cursor >= lines.len() {
                        break lines.len();
                    }
                };
                if !content.is_empty() {
                    blocks.push(DocCommentBlock {
                        start_line: start,
                        end_line: end,
                        target_line_hint: next_nonempty_line(source, end),
                        lines: content,
                    });
                }
            }
        }

        Ok(blocks)
    }

    fn attach_docs(&self, symbols: Vec<DocSymbol>, docs: Vec<DocCommentBlock>) -> Vec<DocSymbol> {
        attach_docs_by_proximity(symbols, docs)
    }
}
