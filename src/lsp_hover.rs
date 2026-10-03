use crate::analyzed_program::AnalyzedProgram;
use crate::interpreter::Interpreter;
use crate::lexer::{Token, TokenKind};
use crate::lsp_definition::{self, DefinitionKind};
use crate::type_checker::ModuleAnalysisCache;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub symbol: String,
    pub kind: String,
    pub detail: String,
    pub line: usize,
    pub column: usize,
}

pub fn hover(source: &str, line: usize, column: usize) -> Option<HoverInfo> {
    let analysis = AnalyzedProgram::analyze(source, None, Arc::new(ModuleAnalysisCache::default()));
    hover_with_analysis(&analysis, line, column)
}

pub fn hover_with_analysis(
    analysis: &AnalyzedProgram,
    line: usize,
    column: usize,
) -> Option<HoverInfo> {
    let source = &analysis.source;
    let tokens = &analysis.tokens;
    let token = identifier_token_at_cursor(&tokens, line, column)?;
    let symbol = match &token.kind {
        TokenKind::Identifier(name) => name.clone(),
        _ => return None,
    };
    let start_column = token.column.saturating_sub(symbol.chars().count());
    if 0 == start_column {
        return None;
    }

    if let Some(receiver) = member_receiver(source, line, start_column) {
        if let Some(crate::ast::TypeAnnotation::Struct(struct_name)) =
            analysis.facts.variables.get(&receiver)
        {
            if let Some(shape) = analysis.facts.structs.get(struct_name) {
                if let Some(ty) = shape.fields.get(&symbol) {
                    return Some(HoverInfo {
                        symbol,
                        kind: "field".into(),
                        detail: format!("{}: {}", token_name(token), ty),
                        line,
                        column: start_column,
                    });
                }
                if let Some(signature) = shape.methods.get(&symbol) {
                    return Some(HoverInfo {
                        symbol: symbol.clone(),
                        kind: "method".into(),
                        detail: signature.display_signature(&symbol),
                        line,
                        column: start_column,
                    });
                }
            }
        } else if let Some(module) = analysis.facts.modules.get(&receiver) {
            if let Some(signature) = module.functions.get(&symbol) {
                return Some(HoverInfo {
                    symbol: symbol.clone(),
                    kind: "function".into(),
                    detail: signature.display_signature(&symbol),
                    line,
                    column: start_column,
                });
            }
            if let Some(ty) = module.values.get(&symbol) {
                return Some(HoverInfo {
                    symbol: symbol.clone(),
                    kind: "variable".into(),
                    detail: format!("{}: {}", symbol, ty),
                    line,
                    column: start_column,
                });
            }
        }
    }

    if let Some(definition) =
        lsp_definition::find_definition_with_tokens(&tokens, line, start_column)
    {
        let is_function = definition.kind == DefinitionKind::Function;
        let mut info = build_user_symbol_hover(
            source,
            &definition.name,
            definition.kind,
            definition.line,
            definition.column,
        );
        if !is_function {
            if let Some(ty) = analysis.facts.variables.get(&symbol) {
                info.detail = format!("{}: {}", symbol, ty);
            }
        }
        return Some(info);
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

    if let Some(signature) = analysis.facts.functions.get(&symbol) {
        return Some(HoverInfo {
            symbol: symbol.clone(),
            kind: "function".into(),
            detail: signature.display_signature(&symbol),
            line,
            column: start_column,
        });
    }
    if let Some(ty) = analysis.facts.variables.get(&symbol) {
        return Some(HoverInfo {
            symbol: symbol.clone(),
            kind: "variable".into(),
            detail: format!("{}: {}", symbol, ty),
            line,
            column: start_column,
        });
    }

    None
}

fn token_name(token: &Token) -> &str {
    match &token.kind {
        TokenKind::Identifier(name) => name,
        _ => "",
    }
}

fn member_receiver(source: &str, line: usize, member_column: usize) -> Option<String> {
    let selected_line = source.lines().nth(line.checked_sub(1)?)?;
    let prefix: String = selected_line.chars().take(member_column.saturating_sub(1)).collect();
    let before_member = prefix.strip_suffix('.')?.trim_end();
    let receiver = before_member
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    (!receiver.is_empty()).then_some(receiver)
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

        if start_column <= column && column < token.column {
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
        assert!(hover(source, 1, 10).is_none());
        assert!(hover(source, 1, 11).is_none());
    }

    #[test]
    fn hover_reports_inferred_struct_field_type() {
        let source =
            "struct User { name: string }\nlet user := User { name: \"Ada\" }\nprint(user.name)\n";
        let info = hover(source, 3, 12).expect("field hover");
        assert_eq!(info.kind, "field");
        assert_eq!(info.detail, "name: String");
    }
}
