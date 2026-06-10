use super::common::{
    attach_docs_by_proximity, extract_jsdoc_comment_blocks, pop_class_stack_for_depth,
    update_brace_depth, visibility_from_explicit_public,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct JavaScriptDocAdapter;

impl JavaScriptDocAdapter {
    fn id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("javascript:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }
}

impl DocLanguageAdapter for JavaScriptDocAdapter {
    fn language_id(&self) -> &'static str {
        "javascript"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "mjs", "cjs"]
    }

    fn capabilities(&self) -> AdapterCapability {
        AdapterCapability {
            supports_functions: true,
            supports_types: false,
            supports_methods: true,
            supports_inline_docs: true,
        }
    }

    fn extract_symbols(&self, source: &str, path: &Path) -> Result<Vec<DocSymbol>, DocgenError> {
        static RE_CLASS: OnceLock<Regex> = OnceLock::new();
        static RE_FUNCTION: OnceLock<Regex> = OnceLock::new();
        static RE_METHOD: OnceLock<Regex> = OnceLock::new();
        let re_class = RE_CLASS.get_or_init(|| {
            Regex::new(r"^\s*(export\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)").expect("js class regex")
        });
        let re_function = RE_FUNCTION.get_or_init(|| {
            Regex::new(r"^\s*(export\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)")
                .expect("js function regex")
        });
        let re_method = RE_METHOD.get_or_init(|| {
            Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*\{").expect("js method regex")
        });

        let mut symbols = Vec::new();
        let mut class_stack: Vec<(String, i32)> = Vec::new();
        let mut depth = 0i32;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            pop_class_stack_for_depth(&mut class_stack, depth);

            if let Some(caps) = re_class.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Class),
                    language: "javascript".to_string(),
                    kind: DocSymbolKind::Class,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                class_stack.push((name, depth + 1));
            } else if let Some(caps) = re_function.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                symbols.push(DocSymbol {
                    id: Self::id(path, line_no, &name, &DocSymbolKind::Function),
                    language: "javascript".to_string(),
                    kind: DocSymbolKind::Function,
                    name: name.clone(),
                    qualified_name: name,
                    signature: Some(format!("function({})", args)),
                    visibility: visibility_from_explicit_public(caps.get(1).is_some()),
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            } else if let Some(caps) = re_method.captures(trimmed) {
                if let Some((class_name, _)) = class_stack.last() {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                    let args = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    if name != "if" && name != "for" && name != "while" && name != "switch" {
                        let qualified = format!("{}.{}", class_name, name);
                        symbols.push(DocSymbol {
                            id: Self::id(path, line_no, &qualified, &DocSymbolKind::Method),
                            language: "javascript".to_string(),
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
                            parent: Some(class_name.clone()),
                        });
                    }
                }
            }

            update_brace_depth(&mut depth, line);
        }

        Ok(symbols)
    }

    fn extract_inline_docs(
        &self,
        source: &str,
        _path: &Path,
    ) -> Result<Vec<DocCommentBlock>, DocgenError> {
        Ok(extract_jsdoc_comment_blocks(source))
    }

    fn attach_docs(&self, symbols: Vec<DocSymbol>, docs: Vec<DocCommentBlock>) -> Vec<DocSymbol> {
        attach_docs_by_proximity(symbols, docs)
    }
}
