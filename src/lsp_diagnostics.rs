use crate::errors::{Diagnostic, DiagnosticSeverity, DiagnosticSubsystem, DIAGNOSTIC_CODE_LSP};
use crate::lexer::{self, LexerDiagnostic, LexerDiagnosticKind, Token, TokenKind};
use crate::parser;

pub fn diagnose(source: &str) -> Vec<Diagnostic> {
    let lexed = lexer::tokenize_with_diagnostics(source);
    let tokens = lexed.tokens;
    let mut parser = parser::Parser::new(tokens.clone());
    let parse_output = parser.parse_with_diagnostics();
    let mut diagnostics = Vec::new();

    diagnostics.extend(lexed.diagnostics.iter().map(to_lsp_diagnostic));
    diagnostics.extend(parse_output.diagnostics.iter().map(to_lsp_parse_diagnostic));
    diagnostics.extend(check_delimiter_balance(&tokens));
    diagnostics.extend(check_type_annotation_syntax(&tokens));

    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
    diagnostics
}

fn to_lsp_diagnostic(diagnostic: &LexerDiagnostic) -> Diagnostic {
    let kind = match diagnostic.kind {
        LexerDiagnosticKind::InvalidCharacter => "Invalid character",
        LexerDiagnosticKind::NullByte => "Null byte",
        LexerDiagnosticKind::UnterminatedString => "Unterminated string",
        LexerDiagnosticKind::UnterminatedComment => "Unterminated comment",
        LexerDiagnosticKind::InvalidEscape => "Invalid escape sequence",
        LexerDiagnosticKind::NumericLiteralOverflow => "Numeric literal overflow",
        LexerDiagnosticKind::MalformedNumericLiteral => "Malformed numeric literal",
        LexerDiagnosticKind::IdentifierTooLong => "Identifier too long",
        LexerDiagnosticKind::StringLiteralTooLong => "String literal too long",
        LexerDiagnosticKind::NumericLiteralTooLong => "Numeric literal too long",
    };

    Diagnostic::new(
        diagnostic.diagnostic_code(),
        DiagnosticSeverity::Error,
        DiagnosticSubsystem::Lexer,
        format!("{}: {}", kind, diagnostic.message),
    )
    .with_location(diagnostic.file.clone(), diagnostic.line, diagnostic.column)
}

fn to_lsp_parse_diagnostic(diagnostic: &parser::ParseDiagnostic) -> Diagnostic {
    diagnostic.to_diagnostic(None)
}

fn check_delimiter_balance(tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut brace_stack: Vec<(usize, usize)> = Vec::new();
    let mut paren_stack: Vec<(usize, usize)> = Vec::new();
    let mut bracket_stack: Vec<(usize, usize)> = Vec::new();

    for token in tokens.iter() {
        match token.kind {
            TokenKind::Punctuation('{') => brace_stack.push((token.line, token.column)),
            TokenKind::Punctuation('}') => {
                if brace_stack.pop().is_none() {
                    diagnostics.push(
                        Diagnostic::new(
                            DIAGNOSTIC_CODE_LSP,
                            DiagnosticSeverity::Error,
                            DiagnosticSubsystem::Lsp,
                            "Unmatched closing brace '}'",
                        )
                        .with_location(None, token.line, token.column),
                    );
                }
            }
            TokenKind::Punctuation('(') => paren_stack.push((token.line, token.column)),
            TokenKind::Punctuation(')') => {
                if paren_stack.pop().is_none() {
                    diagnostics.push(
                        Diagnostic::new(
                            DIAGNOSTIC_CODE_LSP,
                            DiagnosticSeverity::Error,
                            DiagnosticSubsystem::Lsp,
                            "Unmatched closing parenthesis ')'",
                        )
                        .with_location(None, token.line, token.column),
                    );
                }
            }
            TokenKind::Punctuation('[') => bracket_stack.push((token.line, token.column)),
            TokenKind::Punctuation(']') => {
                if bracket_stack.pop().is_none() {
                    diagnostics.push(
                        Diagnostic::new(
                            DIAGNOSTIC_CODE_LSP,
                            DiagnosticSeverity::Error,
                            DiagnosticSubsystem::Lsp,
                            "Unmatched closing bracket ']'",
                        )
                        .with_location(None, token.line, token.column),
                    );
                }
            }
            _ => {}
        }
    }

    for (line, column) in brace_stack.into_iter() {
        diagnostics.push(
            Diagnostic::new(
                DIAGNOSTIC_CODE_LSP,
                DiagnosticSeverity::Error,
                DiagnosticSubsystem::Lsp,
                "Unclosed opening brace '{'",
            )
            .with_location(None, line, column),
        );
    }
    for (line, column) in paren_stack.into_iter() {
        diagnostics.push(
            Diagnostic::new(
                DIAGNOSTIC_CODE_LSP,
                DiagnosticSeverity::Error,
                DiagnosticSubsystem::Lsp,
                "Unclosed opening parenthesis '('",
            )
            .with_location(None, line, column),
        );
    }
    for (line, column) in bracket_stack.into_iter() {
        diagnostics.push(
            Diagnostic::new(
                DIAGNOSTIC_CODE_LSP,
                DiagnosticSeverity::Error,
                DiagnosticSubsystem::Lsp,
                "Unclosed opening bracket '['",
            )
            .with_location(None, line, column),
        );
    }

    diagnostics
}

fn check_type_annotation_syntax(tokens: &[Token]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        if !is_symbol(token, "Result") && !is_symbol(token, "Option") {
            continue;
        }

        if !matches!(
            tokens.get(index + 1).map(|token| &token.kind),
            Some(TokenKind::Operator(op)) if op == "<"
        ) {
            continue;
        }

        let mut depth = 0usize;
        let mut saw_top_level_comma = false;
        let mut closed = false;

        for generic_token in tokens.iter().skip(index + 1) {
            match &generic_token.kind {
                TokenKind::Operator(op) if op == "<" => depth += 1,
                TokenKind::Operator(op) if op == ">" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        closed = true;
                        break;
                    }
                }
                TokenKind::Punctuation(',') if depth == 1 => {
                    saw_top_level_comma = true;
                }
                _ => {}
            }
        }

        if is_symbol(token, "Result") && !saw_top_level_comma {
            diagnostics.push(
                Diagnostic::new(
                    DIAGNOSTIC_CODE_LSP,
                    DiagnosticSeverity::Error,
                    DiagnosticSubsystem::Lsp,
                    "Expected ',' in Result<T, E> type",
                )
                .with_location(None, token.line, token.column),
            );
        } else if !closed {
            let type_name = if is_symbol(token, "Result") { "Result<T, E>" } else { "Option<T>" };
            diagnostics.push(
                Diagnostic::new(
                    DIAGNOSTIC_CODE_LSP,
                    DiagnosticSeverity::Error,
                    DiagnosticSubsystem::Lsp,
                    format!("Expected '>' in {} type", type_name),
                )
                .with_location(None, token.line, token.column),
            );
        }
    }

    diagnostics
}

fn is_symbol(token: &Token, name: &str) -> bool {
    matches!(&token.kind, TokenKind::Identifier(value) | TokenKind::Keyword(value) if value == name)
}

#[cfg(test)]
mod tests {
    use super::diagnose;
    use crate::errors::DiagnosticSeverity;

    #[test]
    fn diagnostics_empty_for_valid_program() {
        let source =
            ["func greet(name) {", "    return name", "}", "print(greet(\"kujo\"))"].join("\n");

        let diagnostics = diagnose(&source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_report_unmatched_closing_brace() {
        let diagnostics = diagnose("}\n");
        assert!(diagnostics.iter().any(|diagnostic| {
            DiagnosticSeverity::Error == diagnostic.severity
                && diagnostic.message.contains("Unmatched closing brace")
        }));
    }

    #[test]
    fn diagnostics_report_unclosed_opening_parenthesis() {
        let diagnostics = diagnose("print((1 + 2)\n");
        assert!(diagnostics.iter().any(|diagnostic| {
            DiagnosticSeverity::Error == diagnostic.severity
                && diagnostic.message.contains("Unclosed opening parenthesis")
        }));
    }

    #[test]
    fn diagnostics_include_lexer_failures() {
        let diagnostics = diagnose("let value := @\n");
        assert!(diagnostics.iter().any(|diagnostic| {
            DiagnosticSeverity::Error == diagnostic.severity
                && diagnostic.message.contains("Invalid character")
                && diagnostic.code.starts_with("KUJOLEX")
        }));
    }

    #[test]
    fn diagnostics_capture_parser_messages() {
        let diagnostics = diagnose("print((1 + 2\n");
        assert!(diagnostics.iter().any(|diagnostic| {
            DiagnosticSeverity::Error == diagnostic.severity
                && diagnostic.message.contains("Expected ')'")
                && diagnostic.code == "KUJOPARSE001"
        }));
    }
}
