use super::common::{
    attach_docs_by_proximity, next_nonempty_line, visibility_from_member_modifier,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct PhpDocAdapter;

impl PhpDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("php:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for PhpDocAdapter {
    fn language_id(&self) -> &'static str {
        "php"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["php"]
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
        static RE_FUNCTION: OnceLock<Regex> = OnceLock::new();
        static RE_CONST: OnceLock<Regex> = OnceLock::new();
        let re_class = RE_CLASS.get_or_init(|| {
            Regex::new(r"^\s*(abstract\s+|final\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)")
                .expect("valid php class regex")
        });
        let re_function = RE_FUNCTION.get_or_init(|| {
            Regex::new(
                r"^\s*(public\s+|private\s+|protected\s+)?(static\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            )
            .expect("valid php function regex")
        });
        let re_const = RE_CONST.get_or_init(|| {
            Regex::new(r"^\s*(public\s+|private\s+|protected\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)")
                .expect("valid php const regex")
        });

        let mut symbols = Vec::new();
        let mut class_stack: Vec<(String, i32)> = Vec::new();
        let mut depth: i32 = 0;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            while class_stack.last().is_some_and(|(_, class_depth)| depth < *class_depth) {
                class_stack.pop();
            }

            if let Some(caps) = re_class.captures(trimmed) {
                let class_name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &class_name, &DocSymbolKind::Class),
                    language: "php".to_string(),
                    kind: DocSymbolKind::Class,
                    name: class_name.clone(),
                    qualified_name: class_name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                if line.contains('{') {
                    class_stack.push((class_name, depth + 1));
                }
            } else if let Some(caps) = re_function.captures(trimmed) {
                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(4).map(|m| m.as_str()).unwrap_or("");
                let parent = class_stack.last().map(|entry| entry.0.clone());
                let kind =
                    if parent.is_some() { DocSymbolKind::Method } else { DocSymbolKind::Function };
                let visibility = visibility_from_member_modifier(
                    caps.get(1).map(|m| m.as_str()),
                    DocVisibility::Public,
                );
                let qualified_name = if let Some(class_name) = &parent {
                    format!("{}::{}", class_name, name)
                } else {
                    name.clone()
                };

                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &qualified_name, &kind),
                    language: "php".to_string(),
                    kind,
                    name,
                    qualified_name,
                    signature: Some(format!("function({})", args)),
                    visibility,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent,
                });
            } else if let Some(caps) = re_const.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Constant),
                    language: "php".to_string(),
                    kind: DocSymbolKind::Constant,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(trimmed.to_string()),
                    visibility: DocVisibility::Public,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: class_stack.last().map(|entry| entry.0.clone()),
                });
            }

            depth += line.chars().filter(|ch| *ch == '{').count() as i32;
            depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
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
            let trimmed = lines[idx].trim();
            if trimmed.starts_with("/**") {
                let start = idx + 1;
                let mut content = Vec::new();
                let mut end = start;
                while idx < lines.len() {
                    let line = lines[idx].trim();
                    let cleaned = line
                        .trim_start_matches("/**")
                        .trim_start_matches("*/")
                        .trim_start_matches('*')
                        .trim()
                        .to_string();
                    if !cleaned.is_empty() {
                        content.push(cleaned);
                    }
                    if line.contains("*/") {
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
