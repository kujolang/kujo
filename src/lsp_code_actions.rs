use crate::lsp_diagnostics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub kind: String,
    pub line: usize,
    pub column: usize,
    pub replacement: String,
    pub description: String,
}

pub fn code_actions(source: &str) -> Vec<CodeAction> {
    let diagnostics = lsp_diagnostics::diagnose(source);
    let source_lines: Vec<&str> = source.lines().collect();

    let mut actions = Vec::new();

    for diagnostic in diagnostics.iter() {
        if diagnostic.message.contains("Unmatched closing brace '}'") {
            actions.push(CodeAction {
                title: "Remove unmatched closing brace".to_string(),
                kind: "quickfix.syntax.remove_unmatched_brace".to_string(),
                line: diagnostic.line,
                column: diagnostic.column,
                replacement: "".to_string(),
                description: "Delete the unmatched closing brace token.".to_string(),
            });
        } else if diagnostic.message.contains("Unclosed opening brace '{'") {
            let insertion_column = line_end_column(&source_lines, diagnostic.line);
            actions.push(CodeAction {
                title: "Insert missing closing brace".to_string(),
                kind: "quickfix.syntax.insert_closing_brace".to_string(),
                line: diagnostic.line,
                column: insertion_column,
                replacement: "}".to_string(),
                description: "Insert a closing brace to balance the opening brace.".to_string(),
            });
        } else if diagnostic.message.contains("Unclosed opening parenthesis '('") {
            let insertion_column = line_end_column(&source_lines, diagnostic.line);
            actions.push(CodeAction {
                title: "Insert missing closing parenthesis".to_string(),
                kind: "quickfix.syntax.insert_closing_parenthesis".to_string(),
                line: diagnostic.line,
                column: insertion_column,
                replacement: ")".to_string(),
                description: "Insert a closing parenthesis to balance the opening parenthesis."
                    .to_string(),
            });
        } else if diagnostic.message.contains("Unclosed opening bracket '['") {
            let insertion_column = line_end_column(&source_lines, diagnostic.line);
            actions.push(CodeAction {
                title: "Insert missing closing bracket".to_string(),
                kind: "quickfix.syntax.insert_closing_bracket".to_string(),
                line: diagnostic.line,
                column: insertion_column,
                replacement: "]".to_string(),
                description: "Insert a closing bracket to balance the opening bracket.".to_string(),
            });
        }
    }

    actions
}

fn line_end_column(lines: &[&str], line: usize) -> usize {
    if 0 == line || line > lines.len() {
        return 1;
    }

    lines[line - 1].chars().count() + 1
}

#[cfg(test)]
mod tests {
    use super::code_actions;

    #[test]
    fn no_actions_for_valid_source() {
        let source = "func greet(name) {\n    return name\n}\n";
        let actions = code_actions(source);
        assert!(actions.is_empty());
    }

    #[test]
    fn action_for_unmatched_closing_brace() {
        let actions = code_actions("}\n");
        assert!(actions
            .iter()
            .any(|action| action.kind == "quickfix.syntax.remove_unmatched_brace"));
    }

    #[test]
    fn action_for_unclosed_parenthesis() {
        let actions = code_actions("print((1 + 2)\n");
        assert!(actions
            .iter()
            .any(|action| action.kind == "quickfix.syntax.insert_closing_parenthesis"));
    }

    #[test]
    fn action_for_unclosed_bracket() {
        let actions = code_actions("let values := [1, 2, 3\n");
        assert!(actions
            .iter()
            .any(|action| action.kind == "quickfix.syntax.insert_closing_bracket"));
    }
}
