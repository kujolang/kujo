use crate::interpreter::Interpreter;
use crate::lexer::{self, Token, TokenKind};
use crate::lsp_definition::{self, DefinitionKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub symbol: String,
    pub kind: String,
    pub detail: String,
    pub line: usize,
    pub column: usize,
}

pub fn hover(source: &str, line: usize, column: usize) -> Option<HoverInfo> {
    let tokens = lexer::tokenize(source).ok()?;
    let token = identifier_token_at_cursor(&tokens, line, column)?;
    let symbol = match &token.kind {
        TokenKind::Identifier(name) => name.clone(),
        _ => return None,
    };
    let start_column = token.column.saturating_sub(symbol.chars().count());
    if 0 == start_column {
        return None;
    }

    if let Some(definition) = lsp_definition::find_definition(source, line, start_column) {
        return Some(build_user_symbol_hover(
            source,
            &definition.name,
            definition.kind,
            definition.line,
            definition.column,
        ));
    }

    if Interpreter::get_builtin_names().iter().any(|name| *name == symbol) {
        return Some(HoverInfo {
            symbol: symbol.clone(),
            kind: "builtin".to_string(),
            detail: format!("Built-in symbol: {}", symbol),
            line,
            column: start_column,
        });
    }

    None
}

fn build_user_symbol_hover(
    source: &str,
    symbol: &str,
    kind: DefinitionKind,
    line: usize,
    column: usize,
) -> HoverInfo {
    let detail = match kind {
        DefinitionKind::Function => {
            let signature = source.lines().nth(line.saturating_sub(1)).unwrap_or("").trim();
            if signature.is_empty() {
                format!("Function: {}", symbol)
            } else {
                format!("Function definition: {}", signature)
            }
        }
        DefinitionKind::Variable => format!("Variable: {}", symbol),
        DefinitionKind::Parameter => format!("Function parameter: {}", symbol),
    };

    HoverInfo { symbol: symbol.to_string(), kind: kind.as_str().to_string(), detail, line, column }
}

fn identifier_token_at_cursor<'a>(
    tokens: &'a [Token],
    line: usize,
    column: usize,
) -> Option<&'a Token> {
    for token in tokens.iter() {
        if token.line != line {
            continue;
        }

        let name = match &token.kind {
            TokenKind::Identifier(name) => name,
            _ => continue,
        };

        let start_column = token.column.saturating_sub(name.chars().count());
        if 0 == start_column {
            continue;
        }

        let end_column = token.column.saturating_sub(1);
        if (start_column..=token.column).contains(&column)
            || (start_column..=end_column).contains(&column)
        {
            return Some(token);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::hover;

    #[test]
    fn hover_returns_function_details_for_user_function() {
        let source =
            ["func greet(name) {", "    return name", "}", "let result := greet(\"kujo\")"]
                .join("\n");

        let info = hover(&source, 4, 16).expect("expected hover info");
        assert_eq!(info.symbol, "greet");
        assert_eq!(info.kind, "function");
        assert_eq!(info.line, 1);
        assert_eq!(info.column, 6);
        assert!(info.detail.contains("func greet(name)"));
    }

    #[test]
    fn hover_returns_builtin_details() {
        let source = "print(1)\n";
        let info = hover(source, 1, 2).expect("expected builtin hover info");

        assert_eq!(info.symbol, "print");
        assert_eq!(info.kind, "builtin");
        assert_eq!(info.line, 1);
        assert_eq!(info.column, 1);
        assert!(info.detail.contains("Built-in symbol"));
    }

    #[test]
    fn hover_returns_parameter_details() {
        let source = ["func square(value) {", "    return value * value", "}"].join("\n");
        let info = hover(&source, 2, 13).expect("expected parameter hover info");

        assert_eq!(info.symbol, "value");
        assert_eq!(info.kind, "parameter");
        assert_eq!(info.line, 1);
        assert_eq!(info.column, 13);
    }

    #[test]
    fn hover_returns_none_when_cursor_not_on_identifier() {
        let source = "let value := 1\n";
        assert!(hover(source, 1, 11).is_none());
    }
}
