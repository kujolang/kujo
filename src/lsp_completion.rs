use crate::ast::{Pattern, Stmt};
use crate::interpreter::Interpreter;
use crate::lexer;
use crate::parser;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    Builtin,
    Function,
    Variable,
}

impl CompletionItemKind {
    fn rank(&self) -> u8 {
        match self {
            CompletionItemKind::Builtin => 1,
            CompletionItemKind::Function => 2,
            CompletionItemKind::Variable => 3,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CompletionItemKind::Builtin => "builtin",
            CompletionItemKind::Function => "function",
            CompletionItemKind::Variable => "variable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
}

pub fn complete(source: &str, line: usize, column: usize) -> Vec<CompletionItem> {
    let prefix = identifier_prefix_before_cursor(source, line, column);
    let mut by_label: BTreeMap<String, CompletionItemKind> = BTreeMap::new();

    for builtin in Interpreter::get_builtin_names() {
        upsert_completion_item(&mut by_label, builtin.to_string(), CompletionItemKind::Builtin);
    }

    let (function_symbols, variable_symbols) = collect_user_symbols(source);

    for function_name in function_symbols {
        upsert_completion_item(&mut by_label, function_name, CompletionItemKind::Function);
    }

    for variable_name in variable_symbols {
        upsert_completion_item(&mut by_label, variable_name, CompletionItemKind::Variable);
    }

    by_label
        .into_iter()
        .filter(|(label, _)| label.starts_with(&prefix))
        .map(|(label, kind)| CompletionItem { label, kind })
        .collect()
}

fn upsert_completion_item(
    by_label: &mut BTreeMap<String, CompletionItemKind>,
    label: String,
    kind: CompletionItemKind,
) {
    match by_label.get(&label) {
        Some(existing_kind) if existing_kind.rank() >= kind.rank() => {}
        _ => {
            by_label.insert(label, kind);
        }
    }
}

fn collect_user_symbols(source: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(_) => return (BTreeSet::new(), BTreeSet::new()),
    };
    let mut parser = parser::Parser::new(tokens);
    let stmts = parser.parse_with_diagnostics().stmts;

    let mut function_symbols = BTreeSet::new();
    let mut variable_symbols = BTreeSet::new();

    for stmt in stmts.iter() {
        collect_symbols_from_stmt(stmt, &mut function_symbols, &mut variable_symbols);
    }

    (function_symbols, variable_symbols)
}

fn collect_symbols_from_stmt(
    stmt: &Stmt,
    function_symbols: &mut BTreeSet<String>,
    variable_symbols: &mut BTreeSet<String>,
) {
    match stmt {
        Stmt::Let { pattern, .. } => {
            collect_pattern_variables(pattern, variable_symbols);
        }
        Stmt::Const { name, .. } => {
            variable_symbols.insert(name.clone());
        }
        Stmt::For { var, body, .. } => {
            variable_symbols.insert(var.clone());
            for child in body.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
        }
        Stmt::FuncDef { name, body, .. } => {
            function_symbols.insert(name.clone());
            for child in body.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
        }
        Stmt::StructDef { methods, .. } => {
            for method_stmt in methods.iter() {
                collect_symbols_from_stmt(method_stmt, function_symbols, variable_symbols);
            }
        }
        Stmt::If { then_branch, else_branch, .. } => {
            for child in then_branch.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
            if let Some(else_statements) = else_branch {
                for child in else_statements.iter() {
                    collect_symbols_from_stmt(child, function_symbols, variable_symbols);
                }
            }
        }
        Stmt::Loop { body, .. }
        | Stmt::While { body, .. }
        | Stmt::Block(body)
        | Stmt::TestSetup { body }
        | Stmt::TestTeardown { body }
        | Stmt::Test { body, .. }
        | Stmt::Spawn { body } => {
            for child in body.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
        }
        Stmt::TestGroup { tests, .. } => {
            for child in tests.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
        }
        Stmt::TryExcept { try_block, except_var, except_block } => {
            variable_symbols.insert(except_var.clone());
            for child in try_block.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
            for child in except_block.iter() {
                collect_symbols_from_stmt(child, function_symbols, variable_symbols);
            }
        }
        Stmt::Export { stmt } => {
            collect_symbols_from_stmt(stmt, function_symbols, variable_symbols);
        }
        Stmt::Match { cases, default, .. } => {
            for (_, case_stmts) in cases.iter() {
                for child in case_stmts.iter() {
                    collect_symbols_from_stmt(child, function_symbols, variable_symbols);
                }
            }
            if let Some(default_stmts) = default {
                for child in default_stmts.iter() {
                    collect_symbols_from_stmt(child, function_symbols, variable_symbols);
                }
            }
        }
        _ => {}
    }
}

fn collect_pattern_variables(pattern: &Pattern, variable_symbols: &mut BTreeSet<String>) {
    match pattern {
        Pattern::Identifier(name) => {
            if name != "_" {
                variable_symbols.insert(name.clone());
            }
        }
        Pattern::Array { elements, rest } => {
            for element in elements.iter() {
                collect_pattern_variables(element, variable_symbols);
            }
            if let Some(rest_name) = rest {
                variable_symbols.insert(rest_name.clone());
            }
        }
        Pattern::Dict { keys, rest } => {
            for key in keys.iter() {
                variable_symbols.insert(key.clone());
            }
            if let Some(rest_name) = rest {
                variable_symbols.insert(rest_name.clone());
            }
        }
        Pattern::Ignore => {}
    }
}

fn identifier_prefix_before_cursor(source: &str, line: usize, column: usize) -> String {
    if line == 0 || column == 0 {
        return String::new();
    }

    let selected_line = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let safe_prefix_char_count = column.saturating_sub(1).min(selected_line.chars().count());

    let line_prefix: String = selected_line.chars().take(safe_prefix_char_count).collect();
    let mut prefix_chars = Vec::new();

    for character in line_prefix.chars().rev() {
        if character.is_ascii_alphanumeric() || character == '_' {
            prefix_chars.push(character);
        } else {
            break;
        }
    }

    prefix_chars.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::{complete, identifier_prefix_before_cursor, CompletionItemKind};

    #[test]
    fn identifier_prefix_tracks_word_before_cursor() {
        let source = "let value := 1\npri\n";
        assert_eq!(identifier_prefix_before_cursor(source, 2, 4), "pri");
        assert_eq!(identifier_prefix_before_cursor(source, 2, 3), "pr");
        assert_eq!(identifier_prefix_before_cursor(source, 2, 1), "");
    }

    #[test]
    fn completion_includes_builtin_function_and_variable_matches() {
        let source = [
            "func compute_total(x) {",
            "    return x",
            "}",
            "let printer := 1",
            "let project_name := \"kujo\"",
            "pr",
            "co",
        ]
        .join("\n");

        let completions = complete(&source, 6, 3);
        let completion_pairs: Vec<(String, CompletionItemKind)> =
            completions.iter().map(|item| (item.label.clone(), item.kind.clone())).collect();

        assert!(completion_pairs.contains(&("print".to_string(), CompletionItemKind::Builtin)));
        assert!(completion_pairs.contains(&("printer".to_string(), CompletionItemKind::Variable)));
        assert!(
            completion_pairs.contains(&("project_name".to_string(), CompletionItemKind::Variable))
        );

        let function_completions = complete(&source, 6, 3);
        assert!(!function_completions.iter().any(|item| item.label == "compute_total"));

        let co_completions = complete(&source, 7, 3);
        assert!(co_completions.iter().any(|item| item.label == "compute_total"));
    }

    #[test]
    fn completion_prefers_user_defined_symbol_kind_on_name_collision() {
        let source = "func print() { return null }\npr\n";
        let completions = complete(source, 2, 3);

        let print_item = completions
            .iter()
            .find(|item| item.label == "print")
            .expect("expected print completion to exist");
        assert_eq!(print_item.kind, CompletionItemKind::Function);
    }
}
