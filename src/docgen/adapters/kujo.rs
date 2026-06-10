use super::common::{
    attach_docs_by_proximity, effective_member_visibility, visibility_from_explicit_public,
    visibility_inherits_from_container,
};
use super::{AdapterCapability, DocLanguageAdapter};
use crate::ast::Stmt;
use crate::docgen::model::{DocComment, DocCommentBlock, DocSymbol, DocSymbolKind, DocVisibility};
use crate::docgen::DocgenError;
use crate::{lexer, parser::Parser};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct KujoDocAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KujoParserAssistedStrategy {
    ParserAssisted,
    RegexFallbackLexerDiagnostics,
    RegexFallbackParserDiagnostics,
}

#[derive(Debug, Clone)]
pub struct KujoParserAssistedExtraction {
    pub symbols: Vec<DocSymbol>,
    #[allow(dead_code)]
    // Accessed by tests and optional diagnostics consumers; core run path currently reads symbols only.
    pub strategy: KujoParserAssistedStrategy,
}

impl KujoDocAdapter {
    fn symbol_id(path: &Path, line: usize, name: &str, kind: &DocSymbolKind) -> String {
        format!("kujo:{}:{}:{}:{:?}", path.display(), line, name, kind)
    }

    fn next_doc_target_line(source: &str, from_line: usize) -> Option<usize> {
        for (idx, line) in source.lines().enumerate().skip(from_line) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('@') || trimmed.starts_with("#[") {
                continue;
            }
            return Some(idx + 1);
        }
        None
    }

    fn extract_symbols_regex_only(
        source: &str,
        path: &Path,
    ) -> Result<Vec<DocSymbol>, DocgenError> {
        KujoDocAdapter.extract_symbols(source, path)
    }

    fn extract_symbols_from_stmt(
        stmt: &Stmt,
        source_lines: &[&str],
        cursor_line: &mut usize,
        parent_struct: Option<(&str, bool)>,
        inherited_public: bool,
        path: &Path,
        symbols: &mut Vec<DocSymbol>,
    ) {
        match stmt {
            Stmt::Export { stmt } => {
                Self::extract_symbols_from_stmt(
                    stmt,
                    source_lines,
                    cursor_line,
                    parent_struct,
                    true,
                    path,
                    symbols,
                );
            }
            Stmt::FuncDef { name, params, is_async, is_generator, .. } => {
                let is_method = parent_struct.is_some();
                let kind = if is_method { DocSymbolKind::Method } else { DocSymbolKind::Function };
                let line = find_function_decl_line(source_lines, *cursor_line, name);
                *cursor_line = line;
                let visibility = if is_method {
                    if let Some((_, struct_is_public)) = parent_struct {
                        visibility_from_explicit_public(struct_is_public)
                    } else {
                        DocVisibility::Private
                    }
                } else {
                    visibility_from_explicit_public(inherited_public)
                };
                let parent_name = parent_struct.map(|(name, _)| name.to_string());
                let qualified_name = if let Some(parent_name) = &parent_name {
                    format!("{}::{}", parent_name, name)
                } else {
                    name.clone()
                };
                let mut signature = String::new();
                if *is_async {
                    signature.push_str("async ");
                }
                signature.push_str(if *is_generator { "func*" } else { "func" });
                signature.push('(');
                signature.push_str(&params.join(", "));
                signature.push(')');
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line, &qualified_name, &kind),
                    language: "kujo".to_string(),
                    kind,
                    name: name.clone(),
                    qualified_name,
                    signature: Some(signature),
                    visibility,
                    source_path: path.to_path_buf(),
                    line,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: parent_name,
                });
            }
            Stmt::StructDef { name, methods, .. } => {
                let line = find_keyword_decl_line(source_lines, *cursor_line, "struct", name);
                *cursor_line = line;
                let struct_is_public = inherited_public;
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line, name, &DocSymbolKind::Struct),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Struct,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(
                        source_lines
                            .get(line.saturating_sub(1))
                            .map(|line| line.trim().to_string())
                            .unwrap_or_else(|| format!("struct {}", name)),
                    ),
                    visibility: visibility_from_explicit_public(struct_is_public),
                    source_path: path.to_path_buf(),
                    line,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                let mut method_cursor = line;
                for method in methods {
                    Self::extract_symbols_from_stmt(
                        method,
                        source_lines,
                        &mut method_cursor,
                        Some((name, struct_is_public)),
                        false,
                        path,
                        symbols,
                    );
                }
                *cursor_line = (*cursor_line).max(method_cursor);
            }
            Stmt::EnumDef { name, variants } => {
                let line = find_keyword_decl_line(source_lines, *cursor_line, "enum", name);
                *cursor_line = line;
                let enum_visibility = visibility_from_explicit_public(inherited_public);
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line, name, &DocSymbolKind::Enum),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Enum,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(
                        source_lines
                            .get(line.saturating_sub(1))
                            .map(|line| line.trim().to_string())
                            .unwrap_or_else(|| format!("enum {}", name)),
                    ),
                    visibility: enum_visibility.clone(),
                    source_path: path.to_path_buf(),
                    line,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                for variant in variants {
                    let variant_line = find_enum_variant_line(source_lines, *cursor_line, variant);
                    *cursor_line = (*cursor_line).max(variant_line);
                    let qualified_name = format!("{}::{}", name, variant);
                    symbols.push(DocSymbol {
                        id: Self::symbol_id(
                            path,
                            variant_line,
                            &qualified_name,
                            &DocSymbolKind::EnumVariant,
                        ),
                        language: "kujo".to_string(),
                        kind: DocSymbolKind::EnumVariant,
                        name: variant.clone(),
                        qualified_name,
                        signature: Some(variant.clone()),
                        visibility: visibility_inherits_from_container(Some(
                            enum_visibility.clone(),
                        )),
                        source_path: path.to_path_buf(),
                        line: variant_line,
                        docs: DocComment::default(),
                        examples: Vec::new(),
                        gaps: Vec::new(),
                        parent: Some(name.clone()),
                    });
                }
            }
            Stmt::Const { name, .. } => {
                let line = find_binding_decl_line(source_lines, *cursor_line, name);
                *cursor_line = line;
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line, name, &DocSymbolKind::Constant),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Constant,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(
                        source_lines
                            .get(line.saturating_sub(1))
                            .map(|line| line.trim().to_string())
                            .unwrap_or_else(|| format!("const {}", name)),
                    ),
                    visibility: visibility_from_explicit_public(inherited_public),
                    source_path: path.to_path_buf(),
                    line,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
            }
            _ => {}
        }
    }
}

fn keyword_decl_matches(trimmed: &str, keyword: &str, name: &str) -> bool {
    let target = format!("{} {}", keyword, name);
    let export_target = format!("export {} {}", keyword, name);
    trimmed.starts_with(&target) || trimmed.starts_with(&export_target)
}

fn find_keyword_decl_line(
    source_lines: &[&str],
    start_line: usize,
    keyword: &str,
    name: &str,
) -> usize {
    for (idx, line) in source_lines.iter().enumerate().skip(start_line.saturating_sub(1)) {
        if keyword_decl_matches(line.trim(), keyword, name) {
            return idx + 1;
        }
    }
    start_line.max(1)
}

fn is_function_declaration_line(trimmed: &str, name: &str) -> bool {
    let func_target = format!("{}(", name);
    let has_callable_head = [
        "func ",
        "func* ",
        "async func ",
        "async func* ",
        "export func ",
        "export func* ",
        "export async func ",
        "export async func* ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix));
    has_callable_head && trimmed.contains(&func_target)
}

fn find_function_decl_line(source_lines: &[&str], start_line: usize, name: &str) -> usize {
    for (idx, line) in source_lines.iter().enumerate().skip(start_line.saturating_sub(1)) {
        if is_function_declaration_line(line.trim(), name) {
            return idx + 1;
        }
    }
    start_line.max(1)
}

fn find_binding_decl_line(source_lines: &[&str], start_line: usize, name: &str) -> usize {
    for (idx, line) in source_lines.iter().enumerate().skip(start_line.saturating_sub(1)) {
        let trimmed = line.trim();
        let base_matches = trimmed.starts_with(&format!("const {}", name))
            || trimmed.starts_with(&format!("let {}", name))
            || trimmed.starts_with(&format!("export const {}", name))
            || trimmed.starts_with(&format!("export let {}", name));
        if base_matches {
            return idx + 1;
        }
    }
    start_line.max(1)
}

fn find_enum_variant_line(source_lines: &[&str], start_line: usize, name: &str) -> usize {
    for (idx, line) in source_lines.iter().enumerate().skip(start_line.saturating_sub(1)) {
        let trimmed = line.trim().trim_end_matches(',');
        if trimmed == name {
            return idx + 1;
        }
    }
    start_line.max(1)
}

pub fn extract_symbols_with_parser_fallback(
    source: &str,
    path: &Path,
) -> Result<KujoParserAssistedExtraction, DocgenError> {
    let lexed = lexer::tokenize_with_diagnostics(source);
    if !lexed.diagnostics.is_empty() {
        return Ok(KujoParserAssistedExtraction {
            symbols: KujoDocAdapter::extract_symbols_regex_only(source, path)?,
            strategy: KujoParserAssistedStrategy::RegexFallbackLexerDiagnostics,
        });
    }

    let mut parser = Parser::new(lexed.tokens);
    let parse_output = parser.parse_with_diagnostics();
    if !parse_output.diagnostics.is_empty() {
        return Ok(KujoParserAssistedExtraction {
            symbols: KujoDocAdapter::extract_symbols_regex_only(source, path)?,
            strategy: KujoParserAssistedStrategy::RegexFallbackParserDiagnostics,
        });
    }

    let mut symbols = Vec::new();
    let source_lines: Vec<&str> = source.lines().collect();
    let mut cursor_line = 1usize;
    for stmt in &parse_output.stmts {
        KujoDocAdapter::extract_symbols_from_stmt(
            stmt,
            &source_lines,
            &mut cursor_line,
            None,
            false,
            path,
            &mut symbols,
        );
    }

    Ok(KujoParserAssistedExtraction {
        symbols,
        strategy: KujoParserAssistedStrategy::ParserAssisted,
    })
}

impl DocLanguageAdapter for KujoDocAdapter {
    fn language_id(&self) -> &'static str {
        "kujo"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["kujo"]
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
        static RE_FUNC: OnceLock<Regex> = OnceLock::new();
        static RE_STRUCT: OnceLock<Regex> = OnceLock::new();
        static RE_ENUM: OnceLock<Regex> = OnceLock::new();
        static RE_CONST: OnceLock<Regex> = OnceLock::new();
        static RE_VARIANT: OnceLock<Regex> = OnceLock::new();
        let re_func = RE_FUNC.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?(async\s+)?func\*?\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)")
                .expect("valid kujo function regex")
        });
        let re_struct = RE_STRUCT.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)")
                .expect("valid kujo struct regex")
        });
        let re_enum = RE_ENUM.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)")
                .expect("valid kujo enum regex")
        });
        let re_const = RE_CONST.get_or_init(|| {
            Regex::new(r"^\s*(pub\s+)?(const|let)\s+([A-Za-z_][A-Za-z0-9_]*)\s*[:=]")
                .expect("valid kujo const regex")
        });
        let re_variant = RE_VARIANT.get_or_init(|| {
            Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*,?\s*$")
                .expect("valid kujo enum variant regex")
        });

        let mut symbols = Vec::new();
        let mut brace_depth: i32 = 0;
        let mut active_struct: Option<(String, i32, bool)> = None;
        let mut active_enum: Option<(String, i32, bool)> = None;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();

            if let Some((_, end_depth, _)) = active_struct.clone() {
                if brace_depth < end_depth {
                    active_struct = None;
                }
            }
            if let Some((_, end_depth, _)) = active_enum.clone() {
                if brace_depth < end_depth {
                    active_enum = None;
                }
            }

            if let Some(caps) = re_struct.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let visibility = visibility_from_explicit_public(caps.get(1).is_some());
                let is_public = visibility == DocVisibility::Public;
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line_no, &name, &DocSymbolKind::Struct),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Struct,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                if line.contains('{') {
                    active_struct = Some((name, brace_depth + 1, is_public));
                }
            } else if let Some(caps) = re_enum.captures(trimmed) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let visibility = visibility_from_explicit_public(caps.get(1).is_some());
                let is_public = visibility == DocVisibility::Public;
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line_no, &name, &DocSymbolKind::Enum),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Enum,
                    name: name.clone(),
                    qualified_name: name.clone(),
                    signature: Some(trimmed.to_string()),
                    visibility,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent: None,
                });
                if line.contains('{') {
                    active_enum = Some((name, brace_depth + 1, is_public));
                }
            } else if let Some(caps) = re_func.captures(trimmed) {
                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let args = caps.get(4).map(|m| m.as_str()).unwrap_or("");
                let is_method = active_struct.is_some();
                let kind = if is_method { DocSymbolKind::Method } else { DocSymbolKind::Function };
                let parent = active_struct.as_ref().map(|(name, _, _)| name.clone());
                let is_async = caps.get(2).is_some();
                let explicit_public = caps.get(1).is_some();
                let declared_visibility = visibility_from_explicit_public(explicit_public);
                let container_visibility = active_struct
                    .as_ref()
                    .map(|(_, _, parent_public)| visibility_from_explicit_public(*parent_public));
                let visibility =
                    effective_member_visibility(declared_visibility, container_visibility, true);
                let qualified_name = if let Some(parent_name) = &parent {
                    format!("{}::{}", parent_name, name)
                } else {
                    name.clone()
                };
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line_no, &qualified_name, &kind),
                    language: "kujo".to_string(),
                    kind,
                    name,
                    qualified_name,
                    signature: Some(if is_async {
                        format!("async func({})", args)
                    } else {
                        format!("func({})", args)
                    }),
                    visibility,
                    source_path: path.to_path_buf(),
                    line: line_no,
                    docs: DocComment::default(),
                    examples: Vec::new(),
                    gaps: Vec::new(),
                    parent,
                });
            } else if let Some(caps) = re_const.captures(trimmed) {
                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown").to_string();
                let visibility = visibility_from_explicit_public(caps.get(1).is_some());
                symbols.push(DocSymbol {
                    id: Self::symbol_id(path, line_no, &name, &DocSymbolKind::Constant),
                    language: "kujo".to_string(),
                    kind: DocSymbolKind::Constant,
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
            } else if let Some((enum_name, _, enum_public)) = active_enum.clone() {
                if re_variant.is_match(trimmed)
                    && !trimmed.starts_with('#')
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with("/*")
                    && !trimmed.starts_with('}')
                {
                    let variant_name = trimmed.trim_end_matches(',').trim().to_string();
                    let qualified = format!("{}::{}", enum_name, variant_name);
                    symbols.push(DocSymbol {
                        id: Self::symbol_id(path, line_no, &qualified, &DocSymbolKind::EnumVariant),
                        language: "kujo".to_string(),
                        kind: DocSymbolKind::EnumVariant,
                        name: variant_name,
                        qualified_name: qualified,
                        signature: Some(trimmed.to_string()),
                        visibility: visibility_inherits_from_container(Some(
                            visibility_from_explicit_public(enum_public),
                        )),
                        source_path: path.to_path_buf(),
                        line: line_no,
                        docs: DocComment::default(),
                        examples: Vec::new(),
                        gaps: Vec::new(),
                        parent: Some(enum_name),
                    });
                }
            }

            let opens = line.chars().filter(|ch| *ch == '{').count() as i32;
            let closes = line.chars().filter(|ch| *ch == '}').count() as i32;
            brace_depth += opens;
            brace_depth -= closes;
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
                let start_line = idx + 1;
                let mut content = Vec::new();
                let mut end_line = start_line;

                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    if !(candidate.starts_with("///") || candidate.starts_with("//!")) {
                        break;
                    }
                    content.push(
                        candidate
                            .trim_start_matches("///")
                            .trim_start_matches("//!")
                            .trim_start()
                            .to_string(),
                    );
                    end_line = idx + 1;
                    idx += 1;
                }

                blocks.push(DocCommentBlock {
                    start_line,
                    end_line,
                    target_line_hint: Self::next_doc_target_line(source, end_line),
                    lines: content,
                });
                continue;
            }

            if trimmed.starts_with("/**") {
                let start_line = idx + 1;
                let mut content = Vec::new();
                let mut end_line = start_line;

                while idx < lines.len() {
                    let candidate = lines[idx].trim();
                    let cleaned = candidate
                        .trim_start_matches("/**")
                        .trim_start_matches("*/")
                        .trim_start_matches('*')
                        .trim()
                        .to_string();
                    if !cleaned.is_empty() {
                        content.push(cleaned);
                    }
                    end_line = idx + 1;
                    if candidate.contains("*/") {
                        idx += 1;
                        break;
                    }
                    idx += 1;
                }

                blocks.push(DocCommentBlock {
                    start_line,
                    end_line: end_line.max(start_line),
                    target_line_hint: Self::next_doc_target_line(source, end_line.max(start_line)),
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
