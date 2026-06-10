use super::common::{attach_docs_by_proximity, next_nonempty_line};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct RubyDocAdapter;

impl RubyDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("ruby:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for RubyDocAdapter {
    fn language_id(&self) -> &'static str {
        "ruby"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["rb"]
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
        static RE_MODULE: OnceLock<Regex> = OnceLock::new();
        static RE_DEF: OnceLock<Regex> = OnceLock::new();
        let re_class = RE_CLASS.get_or_init(|| {
            Regex::new(r"^\s*class\s+([A-Za-z_][A-Za-z0-9_:]*)").expect("ruby class regex")
        });
        let re_module = RE_MODULE.get_or_init(|| {
            Regex::new(r"^\s*module\s+([A-Za-z_][A-Za-z0-9_:]*)").expect("ruby module regex")
        });
        let re_def = RE_DEF.get_or_init(|| {
            Regex::new(r"^\s*def\s+([A-Za-z_][A-Za-z0-9_!?=]*)\s*(\(([^)]*)\))?")
                .expect("ruby def regex")
        });

        let mut symbols = Vec::new();
        let mut container_stack: Vec<String> = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if let Some(caps) = re_module.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Module),
                    language: "ruby".to_string(),
                    kind: DocSymbolKind::Module,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: container_stack.last().cloned(),
                });
                container_stack.push(name);
            } else if let Some(caps) = re_class.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Class),
                    language: "ruby".to_string(),
                    kind: DocSymbolKind::Class,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: container_stack.last().cloned(),
                });
                container_stack.push(name);
            } else if let Some(caps) = re_def.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let parent = container_stack.last().cloned();
                let kind =
                    if parent.is_some() { DocSymbolKind::Method } else { DocSymbolKind::Function };
                let qualified = if let Some(container) = &parent {
                    format!("{}#{}", container, name)
                } else {
                    name.clone()
                };
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &qualified, &kind),
                    language: "ruby".to_string(),
                    kind,
                    name,
                    qualified_name: qualified,
                    signature: Some(format!("def({})", args)),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent,
                });
            } else if trimmed == "end" {
                container_stack.pop();
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

        let mut idx = 0;
        while idx < lines.len() {
            let line_no = idx + 1;
            let trimmed = lines[idx].trim();

            if trimmed.starts_with('#') {
                let start = line_no;
                let mut comments = Vec::new();
                let mut end = line_no;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if !candidate.starts_with('#') {
                        break;
                    }
                    comments.push(candidate.trim_start_matches('#').trim().to_string());
                    end = idx + 1;
                    idx += 1;
                }
                blocks.push(DocCommentBlock {
                    start_line: start,
                    end_line: end,
                    target_line_hint: next_nonempty_line(source, end),
                    lines: comments,
                });
                continue;
            }

            if trimmed == "=begin" {
                let start = line_no;
                let mut content = Vec::new();
                let mut end = line_no;
                idx += 1;
                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if candidate == "=end" {
                        end = idx + 1;
                        break;
                    }
                    if !candidate.is_empty() {
                        content.push(candidate.to_string());
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
