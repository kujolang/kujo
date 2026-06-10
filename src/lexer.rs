// File: src/lexer.rs
//
// Lexical analyzer (tokenizer) for the Kujo programming language.
// Converts source code text into a stream of tokens for parsing.

use crate::errors::{Diagnostic, DiagnosticSeverity, DiagnosticSubsystem, DIAGNOSTIC_CODE_LEXER};
use crate::runtime_limits;
use std::num::IntErrorKind;

pub const MAX_IDENTIFIER_LENGTH: usize = 256;
pub const MAX_STRING_LITERAL_LENGTH: usize = runtime_limits::DEFAULT_MAX_STRING_LITERAL_LENGTH;
pub const MAX_NUMERIC_LITERAL_LENGTH: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Int(i64),
    Float(f64),
    String(String),
    InterpolatedString(Vec<InterpolatedPart>),
    Bool(bool),
    Operator(String),
    Punctuation(char),
    Keyword(String),
    Eof,
}

/// Represents parts of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedPart {
    Text(String),
    Expression(String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    #[allow(dead_code)]
    pub line: usize,
    #[allow(dead_code)]
    pub column: usize,
    #[allow(dead_code)]
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerDiagnosticKind {
    InvalidCharacter,
    NullByte,
    UnterminatedString,
    UnterminatedComment,
    InvalidEscape,
    NumericLiteralOverflow,
    MalformedNumericLiteral,
    IdentifierTooLong,
    StringLiteralTooLong,
    NumericLiteralTooLong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerDiagnostic {
    pub kind: LexerDiagnosticKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
    pub file: Option<String>,
}

impl LexerDiagnosticKind {
    fn diagnostic_suffix(&self) -> &'static str {
        match self {
            LexerDiagnosticKind::InvalidCharacter => "001",
            LexerDiagnosticKind::NullByte => "002",
            LexerDiagnosticKind::UnterminatedString => "003",
            LexerDiagnosticKind::UnterminatedComment => "004",
            LexerDiagnosticKind::InvalidEscape => "005",
            LexerDiagnosticKind::NumericLiteralOverflow => "006",
            LexerDiagnosticKind::MalformedNumericLiteral => "007",
            LexerDiagnosticKind::IdentifierTooLong => "008",
            LexerDiagnosticKind::StringLiteralTooLong => "009",
            LexerDiagnosticKind::NumericLiteralTooLong => "010",
        }
    }
}

impl LexerDiagnostic {
    pub fn diagnostic_code(&self) -> String {
        let prefix = DIAGNOSTIC_CODE_LEXER.trim_end_matches("001");
        format!("{}{}", prefix, self.kind.diagnostic_suffix())
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::new(
            self.diagnostic_code(),
            DiagnosticSeverity::Error,
            DiagnosticSubsystem::Lexer,
            self.message.clone(),
        )
        .with_help("Fix the lexical error in source and run again.")
        .with_location(self.file.clone(), self.line, self.column)
    }
}

#[derive(Debug, Clone)]
pub struct LexOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<LexerDiagnostic>,
}

impl LexOutput {
    fn into_result(self) -> Result<Vec<Token>, Vec<LexerDiagnostic>> {
        if self.diagnostics.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.diagnostics)
        }
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LexerDiagnostic>> {
    tokenize_with_file(source, None)
}

pub fn tokenize_with_file(
    source: &str,
    file: Option<&str>,
) -> Result<Vec<Token>, Vec<LexerDiagnostic>> {
    tokenize_with_diagnostics_and_file(source, file).into_result()
}

pub fn tokenize_with_diagnostics(source: &str) -> LexOutput {
    tokenize_with_diagnostics_and_file(source, None)
}

fn tokenize_with_diagnostics_and_file(source: &str, file: Option<&str>) -> LexOutput {
    let chars: Vec<char> = source.chars().collect();
    let mut offsets = Vec::with_capacity(chars.len());
    let mut byte_offset = 0usize;
    for ch in &chars {
        offsets.push(byte_offset);
        byte_offset += ch.len_utf8();
    }

    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut idx = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    fn current_offset(offsets: &[usize], idx: usize, source_len: usize) -> usize {
        offsets.get(idx).copied().unwrap_or(source_len)
    }

    fn push_diag(
        diagnostics: &mut Vec<LexerDiagnostic>,
        kind: LexerDiagnosticKind,
        message: String,
        line: usize,
        column: usize,
        byte_offset: usize,
        file: Option<&str>,
    ) {
        diagnostics.push(LexerDiagnostic {
            kind,
            message,
            line,
            column,
            byte_offset,
            file: file.map(|value| value.to_string()),
        });
    }

    fn push_token(
        tokens: &mut Vec<Token>,
        kind: TokenKind,
        line: usize,
        column: usize,
        byte_offset: usize,
    ) {
        tokens.push(Token { kind, line, column, byte_offset });
    }

    fn bump(chars: &[char], idx: &mut usize) -> Option<char> {
        let ch = chars.get(*idx).copied()?;
        *idx += 1;
        Some(ch)
    }

    fn peek(chars: &[char], idx: usize) -> Option<char> {
        chars.get(idx).copied()
    }

    fn advance_position(ch: char, line: &mut usize, col: &mut usize) {
        if ch == '\n' || ch == '\r' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
    }

    while let Some(c) = peek(&chars, idx) {
        match c {
            ' ' | '\t' => {
                let ch = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(ch, &mut line, &mut col);
            }
            '\n' => {
                let ch = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(ch, &mut line, &mut col);
            }
            '\r' => {
                let ch = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(ch, &mut line, &mut col);
                if peek(&chars, idx) == Some('\n') {
                    // Treat CRLF as a single newline.
                    bump(&chars, &mut idx);
                }
            }
            '\0' => {
                let diagnostic_line = line;
                let diagnostic_col = col;
                let offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('\0', &mut line, &mut col);
                push_diag(
                    &mut diagnostics,
                    LexerDiagnosticKind::NullByte,
                    "Null byte is not allowed in source".to_string(),
                    diagnostic_line,
                    diagnostic_col,
                    offset,
                    file,
                );
            }
            '#' => {
                while let Some(ch) = peek(&chars, idx) {
                    bump(&chars, &mut idx);
                    advance_position(ch, &mut line, &mut col);
                    if ch == '\n' || ch == '\r' {
                        if ch == '\r' && peek(&chars, idx) == Some('\n') {
                            bump(&chars, &mut idx);
                        }
                        break;
                    }
                }
            }
            '"' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('"', &mut line, &mut col);

                let mut parts = Vec::new();
                let mut current_text = String::new();
                let mut has_interpolation = false;
                let mut terminated = false;
                let mut string_error = false;
                let mut string_too_long_reported = false;

                while let Some(ch) = peek(&chars, idx) {
                    if ch == '"' {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);
                        terminated = true;
                        break;
                    }

                    if ch == '\\' {
                        bump(&chars, &mut idx);
                        advance_position('\\', &mut line, &mut col);
                        let escape_line = line;
                        let escape_col = col;
                        let escape_offset = current_offset(&offsets, idx, source.len());
                        if let Some(esc) = peek(&chars, idx) {
                            bump(&chars, &mut idx);
                            advance_position(esc, &mut line, &mut col);
                            match esc {
                                'n' => current_text.push('\n'),
                                't' => current_text.push('\t'),
                                'r' => current_text.push('\r'),
                                '\\' => current_text.push('\\'),
                                '"' => current_text.push('"'),
                                '$' => current_text.push('$'),
                                _ => {
                                    string_error = true;
                                    push_diag(
                                        &mut diagnostics,
                                        LexerDiagnosticKind::InvalidEscape,
                                        format!("Invalid escape sequence: \\\\{}", esc),
                                        escape_line,
                                        escape_col,
                                        escape_offset,
                                        file,
                                    );
                                }
                            }
                        } else {
                            string_error = true;
                            push_diag(
                                &mut diagnostics,
                                LexerDiagnosticKind::UnterminatedString,
                                "Unterminated string literal".to_string(),
                                start_line,
                                start_col,
                                start_offset,
                                file,
                            );
                            break;
                        }
                    } else if ch == '$' && peek(&chars, idx + 1) == Some('{') {
                        has_interpolation = true;
                        bump(&chars, &mut idx);
                        advance_position('$', &mut line, &mut col);
                        bump(&chars, &mut idx);
                        advance_position('{', &mut line, &mut col);

                        if !current_text.is_empty() {
                            parts.push(InterpolatedPart::Text(std::mem::take(&mut current_text)));
                        }

                        let mut expr = String::new();
                        let mut brace_depth = 1usize;
                        let mut interpolation_closed = false;
                        while let Some(inner) = peek(&chars, idx) {
                            bump(&chars, &mut idx);
                            advance_position(inner, &mut line, &mut col);
                            if inner == '{' {
                                brace_depth += 1;
                                expr.push(inner);
                            } else if inner == '}' {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    interpolation_closed = true;
                                    break;
                                }
                                expr.push(inner);
                            } else {
                                expr.push(inner);
                            }
                        }

                        if !interpolation_closed {
                            string_error = true;
                            push_diag(
                                &mut diagnostics,
                                LexerDiagnosticKind::UnterminatedString,
                                "Unterminated interpolated string expression".to_string(),
                                start_line,
                                start_col,
                                start_offset,
                                file,
                            );
                            break;
                        }

                        parts.push(InterpolatedPart::Expression(expr));
                    } else if ch == '\n' || ch == '\r' {
                        string_error = true;
                        push_diag(
                            &mut diagnostics,
                            LexerDiagnosticKind::UnterminatedString,
                            "Unterminated string literal".to_string(),
                            start_line,
                            start_col,
                            start_offset,
                            file,
                        );
                        break;
                    } else {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);
                        current_text.push(ch);

                        if current_text.chars().count() > MAX_STRING_LITERAL_LENGTH
                            && !string_too_long_reported
                        {
                            string_too_long_reported = true;
                            string_error = true;
                            push_diag(
                                &mut diagnostics,
                                LexerDiagnosticKind::StringLiteralTooLong,
                                format!(
                                    "String literal exceeds max length of {} characters",
                                    MAX_STRING_LITERAL_LENGTH
                                ),
                                start_line,
                                start_col,
                                start_offset,
                                file,
                            );
                            // Continue consuming until closing quote for recovery.
                        }
                    }
                }

                if !terminated {
                    string_error = true;
                    if diagnostics.last().map(|d| &d.kind)
                        != Some(&LexerDiagnosticKind::UnterminatedString)
                    {
                        push_diag(
                            &mut diagnostics,
                            LexerDiagnosticKind::UnterminatedString,
                            "Unterminated string literal".to_string(),
                            start_line,
                            start_col,
                            start_offset,
                            file,
                        );
                    }
                }

                if !current_text.is_empty() {
                    parts.push(InterpolatedPart::Text(current_text));
                }

                if !string_error {
                    if has_interpolation {
                        push_token(
                            &mut tokens,
                            TokenKind::InterpolatedString(parts),
                            start_line,
                            start_col,
                            start_offset,
                        );
                    } else {
                        let text = match parts.first() {
                            Some(InterpolatedPart::Text(value)) => value.clone(),
                            _ => String::new(),
                        };
                        push_token(
                            &mut tokens,
                            TokenKind::String(text),
                            start_line,
                            start_col,
                            start_offset,
                        );
                    }
                }
            }
            '0'..='9' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                let mut num = String::new();
                let mut has_decimal = false;

                while let Some(ch) = peek(&chars, idx) {
                    if ch.is_ascii_digit() {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);
                        num.push(ch);
                    } else if ch == '.' && !has_decimal {
                        if matches!(peek(&chars, idx + 1), Some(next) if next.is_ascii_digit()) {
                            bump(&chars, &mut idx);
                            advance_position(ch, &mut line, &mut col);
                            num.push(ch);
                            has_decimal = true;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if num.chars().count() > MAX_NUMERIC_LITERAL_LENGTH {
                    push_diag(
                        &mut diagnostics,
                        LexerDiagnosticKind::NumericLiteralTooLong,
                        format!(
                            "Numeric literal exceeds max length of {} characters",
                            MAX_NUMERIC_LITERAL_LENGTH
                        ),
                        start_line,
                        start_col,
                        start_offset,
                        file,
                    );
                    continue;
                }

                if matches!(peek(&chars, idx), Some(ch) if ch.is_alphabetic() || ch == '_') {
                    let mut suffix = String::new();
                    while let Some(ch) = peek(&chars, idx) {
                        if ch.is_alphanumeric() || ch == '_' {
                            bump(&chars, &mut idx);
                            advance_position(ch, &mut line, &mut col);
                            suffix.push(ch);
                        } else {
                            break;
                        }
                    }
                    push_diag(
                        &mut diagnostics,
                        LexerDiagnosticKind::MalformedNumericLiteral,
                        format!("Malformed numeric literal: {}{}", num, suffix),
                        start_line,
                        start_col,
                        start_offset,
                        file,
                    );
                    continue;
                }

                if has_decimal {
                    match num.parse::<f64>() {
                        Ok(parsed) if parsed.is_finite() => push_token(
                            &mut tokens,
                            TokenKind::Float(parsed),
                            start_line,
                            start_col,
                            start_offset,
                        ),
                        Ok(_) => push_diag(
                            &mut diagnostics,
                            LexerDiagnosticKind::NumericLiteralOverflow,
                            format!("Numeric literal overflow: {}", num),
                            start_line,
                            start_col,
                            start_offset,
                            file,
                        ),
                        Err(error) => {
                            let message = error.to_string();
                            let kind = if message.contains("too large")
                                || message.contains("out of range")
                            {
                                LexerDiagnosticKind::NumericLiteralOverflow
                            } else {
                                LexerDiagnosticKind::MalformedNumericLiteral
                            };
                            push_diag(
                                &mut diagnostics,
                                kind,
                                format!("Malformed numeric literal: {}", num),
                                start_line,
                                start_col,
                                start_offset,
                                file,
                            );
                        }
                    }
                } else {
                    match num.parse::<i64>() {
                        Ok(parsed) => push_token(
                            &mut tokens,
                            TokenKind::Int(parsed),
                            start_line,
                            start_col,
                            start_offset,
                        ),
                        Err(error) => {
                            let kind = match error.kind() {
                                IntErrorKind::PosOverflow | IntErrorKind::NegOverflow => {
                                    LexerDiagnosticKind::NumericLiteralOverflow
                                }
                                _ => LexerDiagnosticKind::MalformedNumericLiteral,
                            };
                            let message = match kind {
                                LexerDiagnosticKind::NumericLiteralOverflow => {
                                    format!("Numeric literal overflow: {}", num)
                                }
                                _ => format!("Malformed numeric literal: {}", num),
                            };
                            push_diag(
                                &mut diagnostics,
                                kind,
                                message,
                                start_line,
                                start_col,
                                start_offset,
                                file,
                            );
                        }
                    }
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                let mut ident = String::new();
                while let Some(ch) = peek(&chars, idx) {
                    if ch.is_alphanumeric() || ch == '_' {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);
                        ident.push(ch);
                    } else {
                        break;
                    }
                }

                if ident.chars().count() > MAX_IDENTIFIER_LENGTH {
                    push_diag(
                        &mut diagnostics,
                        LexerDiagnosticKind::IdentifierTooLong,
                        format!(
                            "Identifier exceeds max length of {} characters",
                            MAX_IDENTIFIER_LENGTH
                        ),
                        start_line,
                        start_col,
                        start_offset,
                        file,
                    );
                    continue;
                }

                let kind = match ident.as_str() {
                    "let" | "mut" | "const" | "func" | "return" | "enum" | "match" | "case"
                    | "default" | "if" | "else" | "loop" | "while" | "for" | "in" | "break"
                    | "continue" | "try" | "except" | "int" | "float" | "string" | "bool"
                    | "import" | "export" | "from" | "struct" | "impl" | "self" | "null"
                    | "spawn" | "test" | "test_setup" | "test_teardown" | "test_group"
                    | "yield" | "async" | "await" => TokenKind::Keyword(ident),
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    _ => TokenKind::Identifier(ident),
                };

                // Preserve legacy column semantics for identifier-like tokens:
                // column points to the character after the token so LSP symbol helpers that
                // compute `start = column - name_len` remain stable.
                push_token(&mut tokens, kind, start_line, col, start_offset);
            }
            ':' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position(':', &mut line, &mut col);
                if peek(&chars, idx) == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator(":=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if peek(&chars, idx) == Some(':') {
                    bump(&chars, &mut idx);
                    advance_position(':', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("::".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Punctuation(':'),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '=' | '+' | '-' | '*' | '<' | '>' | '!' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                let op = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(op, &mut line, &mut col);
                let maybe_next = peek(&chars, idx);
                if op == '=' && maybe_next == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("==".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if op == '!' && maybe_next == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("!=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if op == '>' && maybe_next == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator(">=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if op == '<' && maybe_next == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("<=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if matches!(op, '+' | '-' | '*') && maybe_next == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator(format!("{}=", op)),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if op == '-' && maybe_next == Some('>') {
                    bump(&chars, &mut idx);
                    advance_position('>', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("->".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator(op.to_string()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '?' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('?', &mut line, &mut col);
                if peek(&chars, idx) == Some('?') {
                    bump(&chars, &mut idx);
                    advance_position('?', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("??".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if peek(&chars, idx) == Some('.') {
                    bump(&chars, &mut idx);
                    advance_position('.', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("?.".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("?".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '/' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('/', &mut line, &mut col);

                if peek(&chars, idx) == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("/=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if peek(&chars, idx) == Some('*') {
                    bump(&chars, &mut idx);
                    advance_position('*', &mut line, &mut col);

                    let mut found_end = false;
                    while let Some(ch) = peek(&chars, idx) {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);

                        if ch == '*' && peek(&chars, idx) == Some('/') {
                            bump(&chars, &mut idx);
                            advance_position('/', &mut line, &mut col);
                            found_end = true;
                            break;
                        }

                        if ch == '\r' && peek(&chars, idx) == Some('\n') {
                            bump(&chars, &mut idx);
                        }
                    }

                    if !found_end {
                        push_diag(
                            &mut diagnostics,
                            LexerDiagnosticKind::UnterminatedComment,
                            "Unterminated block comment".to_string(),
                            start_line,
                            start_col,
                            start_offset,
                            file,
                        );
                    }
                } else if peek(&chars, idx) == Some('/') {
                    bump(&chars, &mut idx);
                    advance_position('/', &mut line, &mut col);
                    while let Some(ch) = peek(&chars, idx) {
                        bump(&chars, &mut idx);
                        advance_position(ch, &mut line, &mut col);
                        if ch == '\n' || ch == '\r' {
                            if ch == '\r' && peek(&chars, idx) == Some('\n') {
                                bump(&chars, &mut idx);
                            }
                            break;
                        }
                    }
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("/".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '%' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('%', &mut line, &mut col);
                if peek(&chars, idx) == Some('=') {
                    bump(&chars, &mut idx);
                    advance_position('=', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("%=".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("%".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '|' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('|', &mut line, &mut col);
                if peek(&chars, idx) == Some('|') {
                    bump(&chars, &mut idx);
                    advance_position('|', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("||".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else if peek(&chars, idx) == Some('>') {
                    bump(&chars, &mut idx);
                    advance_position('>', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("|>".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("|".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '&' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('&', &mut line, &mut col);
                if peek(&chars, idx) == Some('&') {
                    bump(&chars, &mut idx);
                    advance_position('&', &mut line, &mut col);
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("&&".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Operator("&".into()),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '.' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                bump(&chars, &mut idx);
                advance_position('.', &mut line, &mut col);
                if peek(&chars, idx) == Some('.') {
                    bump(&chars, &mut idx);
                    advance_position('.', &mut line, &mut col);
                    if peek(&chars, idx) == Some('.') {
                        bump(&chars, &mut idx);
                        advance_position('.', &mut line, &mut col);
                        push_token(
                            &mut tokens,
                            TokenKind::Operator("...".into()),
                            start_line,
                            start_col,
                            start_offset,
                        );
                    } else {
                        push_token(
                            &mut tokens,
                            TokenKind::Punctuation('.'),
                            start_line,
                            start_col,
                            start_offset,
                        );
                        push_token(
                            &mut tokens,
                            TokenKind::Punctuation('.'),
                            start_line,
                            start_col + 1,
                            start_offset.saturating_add(1),
                        );
                    }
                } else {
                    push_token(
                        &mut tokens,
                        TokenKind::Punctuation('.'),
                        start_line,
                        start_col,
                        start_offset,
                    );
                }
            }
            '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' => {
                let start_line = line;
                let start_col = col;
                let start_offset = current_offset(&offsets, idx, source.len());
                let ch = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(ch, &mut line, &mut col);
                push_token(
                    &mut tokens,
                    TokenKind::Punctuation(ch),
                    start_line,
                    start_col,
                    start_offset,
                );
            }
            _ => {
                let diagnostic_line = line;
                let diagnostic_col = col;
                let offset = current_offset(&offsets, idx, source.len());
                let ch = bump(&chars, &mut idx).expect("peeked char should exist");
                advance_position(ch, &mut line, &mut col);
                push_diag(
                    &mut diagnostics,
                    LexerDiagnosticKind::InvalidCharacter,
                    format!("Invalid character: '{}'", ch),
                    diagnostic_line,
                    diagnostic_col,
                    offset,
                    file,
                );
            }
        }
    }

    push_token(&mut tokens, TokenKind::Eof, line, col, source.len());

    LexOutput { tokens, diagnostics }
}

#[cfg(test)]
mod tests {
    use super::{
        tokenize, tokenize_with_diagnostics, tokenize_with_file, LexerDiagnosticKind, TokenKind,
        MAX_IDENTIFIER_LENGTH, MAX_NUMERIC_LITERAL_LENGTH, MAX_STRING_LITERAL_LENGTH,
    };

    #[test]
    fn invalid_character_reports_diagnostic() {
        let result = tokenize("let x := @");
        let diagnostics = result.expect_err("expected lexical diagnostics");
        assert!(diagnostics
            .iter()
            .any(|d| d.kind == LexerDiagnosticKind::InvalidCharacter && d.message.contains("@")));
    }

    #[test]
    fn unterminated_string_reports_diagnostic() {
        let result = tokenize("let x := \"hello");
        let diagnostics = result.expect_err("expected unterminated string diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::UnterminatedString));
    }

    #[test]
    fn unterminated_block_comment_reports_diagnostic() {
        let result = tokenize("/* comment");
        let diagnostics = result.expect_err("expected unterminated comment diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::UnterminatedComment));
    }

    #[test]
    fn invalid_escape_reports_diagnostic() {
        let result = tokenize("let x := \"bad\\q\"");
        let diagnostics = result.expect_err("expected invalid escape diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::InvalidEscape));
    }

    #[test]
    fn huge_integer_reports_overflow_diagnostic() {
        let huge = "9".repeat(MAX_NUMERIC_LITERAL_LENGTH + 1);
        let result = tokenize(&huge);
        let diagnostics = result.expect_err("expected numeric diagnostic");
        assert!(diagnostics.iter().any(|d| {
            d.kind == LexerDiagnosticKind::NumericLiteralTooLong
                || d.kind == LexerDiagnosticKind::NumericLiteralOverflow
        }));
    }

    #[test]
    fn huge_float_reports_overflow_diagnostic() {
        let huge = format!("{}.{}", "9".repeat(400), "9".repeat(400));
        let result = tokenize(&huge);
        let diagnostics = result.expect_err("expected float diagnostic");
        assert!(diagnostics.iter().any(|d| {
            d.kind == LexerDiagnosticKind::NumericLiteralTooLong
                || d.kind == LexerDiagnosticKind::NumericLiteralOverflow
                || d.kind == LexerDiagnosticKind::MalformedNumericLiteral
        }));
    }

    #[test]
    fn null_byte_reports_diagnostic() {
        let result = tokenize("let x := \0");
        let diagnostics = result.expect_err("expected null byte diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::NullByte));
    }

    #[test]
    fn mixed_line_endings_preserve_line_column() {
        let output = tokenize_with_diagnostics("ok\r\n@\rnext\n@");
        let invalids: Vec<_> = output
            .diagnostics
            .iter()
            .filter(|d| d.kind == LexerDiagnosticKind::InvalidCharacter)
            .collect();
        assert_eq!(invalids.len(), 2);
        assert_eq!((invalids[0].line, invalids[0].column), (2, 1));
        assert_eq!((invalids[1].line, invalids[1].column), (4, 1));
    }

    #[test]
    fn long_identifier_reports_limit_diagnostic() {
        let ident = "a".repeat(MAX_IDENTIFIER_LENGTH + 1);
        let result = tokenize(&ident);
        let diagnostics = result.expect_err("expected identifier limit diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::IdentifierTooLong));
    }

    #[test]
    fn long_string_reports_limit_diagnostic() {
        let content = "a".repeat(MAX_STRING_LITERAL_LENGTH + 1);
        let source = format!("\"{}\"", content);
        let result = tokenize(&source);
        let diagnostics = result.expect_err("expected string limit diagnostic");
        assert!(diagnostics.iter().any(|d| d.kind == LexerDiagnosticKind::StringLiteralTooLong));
    }

    #[test]
    fn valid_program_still_tokenizes() {
        let tokens = tokenize("let x := 42\nprint(x)").expect("valid source should tokenize");
        assert!(tokens.iter().any(|token| matches!(token.kind, TokenKind::Int(42))));
        assert!(matches!(tokens.last().map(|token| &token.kind), Some(TokenKind::Eof)));
    }

    #[test]
    fn tokenize_with_file_attaches_file_path() {
        let diagnostics = tokenize_with_file("let x := @", Some("example.kujo"))
            .expect_err("expected diagnostic");
        assert_eq!(diagnostics[0].file.as_deref(), Some("example.kujo"));
    }

    #[test]
    fn tokenizes_compound_assignment_operators() {
        let tokens = tokenize("a += 1\nb -= 2\nc *= 3\nd /= 4\ne %= 5")
            .expect("compound assignment source should tokenize");
        let operators: Vec<&str> = tokens
            .iter()
            .filter_map(|token| match &token.kind {
                TokenKind::Operator(operator) => Some(operator.as_str()),
                _ => None,
            })
            .collect();

        assert!(operators.contains(&"+="));
        assert!(operators.contains(&"-="));
        assert!(operators.contains(&"*="));
        assert!(operators.contains(&"/="));
        assert!(operators.contains(&"%="));
    }

    #[test]
    fn token_byte_offsets_are_monotonic() {
        let tokens = tokenize("let value := 42\nprint(value)\n").expect("source should tokenize");
        for pair in tokens.windows(2) {
            assert!(pair[1].byte_offset >= pair[0].byte_offset);
        }
    }
}
