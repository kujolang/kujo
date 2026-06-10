// File: src/parser.rs
//
// Recursive descent parser for the Kujo programming language.
// Transforms a sequence of tokens into an Abstract Syntax Tree (AST).
//
// The parser implements a traditional recursive descent parsing strategy with
// operator precedence for expressions. It supports:
// - Variable declarations (let, mut, const, shorthand :=)
// - Function definitions
// - Enum definitions
// - Control flow (if/else, match, loop, for, try/except)
// - Expression parsing with proper operator precedence
//
// The parser uses a single-token lookahead and advances through the token stream
// as it builds the AST.

use crate::ast::{Expr, Stmt};
use crate::errors::{
    Diagnostic, DiagnosticSeverity, DiagnosticSubsystem, SourceLocation, SourceSpan,
    DIAGNOSTIC_CODE_PARSER,
};
use crate::lexer::{Token, TokenKind};
use crate::runtime_limits;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub const DEFAULT_MAX_SOURCE_BYTES: usize = runtime_limits::DEFAULT_MAX_SOURCE_BYTES;
pub const DEFAULT_MAX_EXPRESSION_DEPTH: usize = runtime_limits::DEFAULT_MAX_EXPRESSION_DEPTH;
pub const DEFAULT_MAX_BLOCK_DEPTH: usize = runtime_limits::DEFAULT_MAX_BLOCK_DEPTH;
pub const DEFAULT_MAX_COLLECTION_LITERAL_ITEMS: usize =
    runtime_limits::DEFAULT_MAX_COLLECTION_LITERAL_ITEMS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub line: usize,
    pub column: usize,
    pub span: SourceSpan,
    pub message: String,
}

impl ParseDiagnostic {
    pub fn to_diagnostic(&self, file: Option<&str>) -> Diagnostic {
        let mut location = self.span.start.clone();
        if let Some(file_name) = file {
            location.file = Some(file_name.to_string());
        }

        Diagnostic::new(
            DIAGNOSTIC_CODE_PARSER,
            DiagnosticSeverity::Error,
            DiagnosticSubsystem::Parser,
            self.message.clone(),
        )
        .with_help("Fix the parse error and rerun Kujo.")
        .with_location(location.file, location.line, location.column)
    }
}

pub fn source_size_limit_diagnostic(
    source_bytes: usize,
    max_source_bytes: usize,
) -> ParseDiagnostic {
    let span = SourceSpan {
        start: SourceLocation::new(1, 1),
        end: SourceLocation::new(1, 1),
        start_byte: 0,
        end_byte: 0,
    };
    ParseDiagnostic {
        line: 1,
        column: 1,
        span,
        message: format!(
            "Source size {} bytes exceeds maximum allowed {} bytes",
            source_bytes, max_source_bytes
        ),
    }
}

pub fn validate_source_size(source: &str, max_source_bytes: usize) -> Result<(), ParseDiagnostic> {
    let source_bytes = source.len();
    if source_bytes > max_source_bytes {
        Err(source_size_limit_diagnostic(source_bytes, max_source_bytes))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ParseOutput {
    pub stmts: Vec<Stmt>,
    pub diagnostics: Vec<ParseDiagnostic>,
    #[allow(dead_code)]
    // Used by parser/LSP contracts and targeted tests; not consumed in every binary flow yet.
    pub ast_spans: Vec<AstNodeSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstNodeSpanKind {
    Statement,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNodeSpan {
    pub kind: AstNodeSpanKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy)]
pub struct ParserLimits {
    pub max_expression_depth: usize,
    pub max_block_depth: usize,
    pub max_collection_literal_items: usize,
}

impl Default for ParserLimits {
    fn default() -> Self {
        Self {
            max_expression_depth: DEFAULT_MAX_EXPRESSION_DEPTH,
            max_block_depth: DEFAULT_MAX_BLOCK_DEPTH,
            max_collection_literal_items: DEFAULT_MAX_COLLECTION_LITERAL_ITEMS,
        }
    }
}

/// Parser maintains position in token stream and provides methods to parse statements and expressions
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<ParseDiagnostic>,
    expression_depth: usize,
    block_depth: usize,
    max_expression_depth: usize,
    max_block_depth: usize,
    max_collection_literal_items: usize,
    ast_spans: Vec<AstNodeSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRuntimeStrategy {
    Interpreter,
    Vm,
    Dual,
}

impl TestRuntimeStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::Interpreter => "interpreter",
            Self::Vm => "vm",
            Self::Dual => "dual",
        }
    }
}

impl Parser {
    /// Creates a new parser from a vector of tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self::new_with_limits(tokens, ParserLimits::default())
    }

    pub fn new_with_limits(tokens: Vec<Token>, limits: ParserLimits) -> Self {
        Parser {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            expression_depth: 0,
            block_depth: 0,
            max_expression_depth: limits.max_expression_depth,
            max_block_depth: limits.max_block_depth,
            max_collection_literal_items: limits.max_collection_literal_items,
            ast_spans: Vec::new(),
        }
    }

    /// Peek at the current token without consuming it
    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    /// Consume and return the current token, then advance to the next
    fn advance(&mut self) -> &TokenKind {
        let tok = self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof);
        self.pos += 1;
        tok
    }

    fn token_width_chars(token: &Token) -> usize {
        match &token.kind {
            TokenKind::Identifier(value)
            | TokenKind::Keyword(value)
            | TokenKind::Operator(value) => value.chars().count().max(1),
            TokenKind::Int(value) => value.to_string().chars().count().max(1),
            TokenKind::Float(value) => value.to_string().chars().count().max(1),
            TokenKind::String(value) => value.chars().count().saturating_add(2).max(1),
            TokenKind::InterpolatedString(_) => 2,
            TokenKind::Bool(true) => 4,
            TokenKind::Bool(false) => 5,
            TokenKind::Punctuation(_) => 1,
            TokenKind::Eof => 0,
        }
    }

    fn token_span_at(&self, pos: usize) -> SourceSpan {
        if let Some(token) = self.tokens.get(pos) {
            let width_chars = Self::token_width_chars(token);
            let uses_end_column = matches!(
                token.kind,
                TokenKind::Identifier(_) | TokenKind::Keyword(_) | TokenKind::Bool(_)
            );
            let raw_start_column = if uses_end_column {
                token.column.saturating_sub(width_chars)
            } else {
                token.column
            };
            let start_column = raw_start_column.max(1);
            let start = SourceLocation::new(token.line, start_column);
            let end = SourceLocation::new(token.line, start_column.saturating_add(width_chars));
            let start_byte = token.byte_offset;
            let end_byte = start_byte.saturating_add(width_chars);
            SourceSpan::new(start, end, start_byte, end_byte)
        } else if let Some(token) = self.tokens.last() {
            let loc = SourceLocation::new(token.line, token.column);
            SourceSpan::new(loc.clone(), loc, token.byte_offset, token.byte_offset)
        } else {
            SourceSpan::unknown()
        }
    }

    fn current_span(&self) -> SourceSpan {
        self.token_span_at(self.pos)
    }

    fn span_between_positions(&self, start_pos: usize, end_pos: usize) -> SourceSpan {
        let start = self.token_span_at(start_pos);
        if end_pos <= start_pos {
            return start;
        }

        let end_token_index = end_pos.saturating_sub(1);
        let end = self.token_span_at(end_token_index);

        SourceSpan::new(start.start.clone(), end.end.clone(), start.start_byte, end.end_byte)
    }

    fn record_ast_span(&mut self, kind: AstNodeSpanKind, start_pos: usize, end_pos: usize) {
        self.ast_spans
            .push(AstNodeSpan { kind, span: self.span_between_positions(start_pos, end_pos) });
    }

    fn push_diagnostic(&mut self, message: impl Into<String>) {
        let span = self.current_span();
        self.diagnostics.push(ParseDiagnostic {
            line: span.start.line,
            column: span.start.column,
            span,
            message: message.into(),
        });
    }

    fn with_expression_depth<T>(
        &mut self,
        context: &str,
        parse: impl FnOnce(&mut Self) -> Option<T>,
    ) -> Option<T> {
        self.expression_depth += 1;
        if self.expression_depth > self.max_expression_depth {
            self.push_diagnostic(format!(
                "Maximum expression nesting depth of {} exceeded while parsing {}",
                self.max_expression_depth, context
            ));
            self.expression_depth = self.expression_depth.saturating_sub(1);
            return None;
        }

        let result = parse(self);
        self.expression_depth = self.expression_depth.saturating_sub(1);
        result
    }

    fn with_block_depth<T>(
        &mut self,
        context: &str,
        parse: impl FnOnce(&mut Self) -> Option<T>,
    ) -> Option<T> {
        self.block_depth += 1;
        if self.block_depth > self.max_block_depth {
            self.push_diagnostic(format!(
                "Maximum block nesting depth of {} exceeded while parsing {}",
                self.max_block_depth, context
            ));
            self.block_depth = self.block_depth.saturating_sub(1);
            return None;
        }

        let result = parse(self);
        self.block_depth = self.block_depth.saturating_sub(1);
        result
    }

    fn parse_statement_block(
        &mut self,
        open_context: &str,
        close_context: &str,
        depth_context: &str,
    ) -> Option<Vec<Stmt>> {
        if !self.expect_punctuation('{', open_context) {
            return None;
        }

        self.with_block_depth(depth_context, |parser| {
            let mut body = Vec::new();
            while !matches!(parser.peek(), TokenKind::Punctuation('}'))
                && !matches!(parser.peek(), TokenKind::Eof)
            {
                if matches!(parser.peek(), TokenKind::Punctuation(';')) {
                    parser.advance();
                    continue;
                }

                if let Some(stmt) = parser.parse_stmt() {
                    body.push(stmt);
                } else {
                    break;
                }
            }

            if !parser.expect_punctuation('}', close_context) {
                return None;
            }

            Some(body)
        })
    }

    fn expect_punctuation(&mut self, ch: char, context: &str) -> bool {
        if matches!(self.peek(), TokenKind::Punctuation(found) if *found == ch) {
            self.advance();
            true
        } else {
            let found = format!("{:?}", self.peek());
            self.push_diagnostic(format!("Expected '{}' {} but found {}", ch, context, found));
            false
        }
    }

    fn assignment_operator(&self) -> Option<&str> {
        match self.peek() {
            TokenKind::Operator(op)
                if matches!(op.as_str(), ":=" | "=" | "+=" | "-=" | "*=" | "/=" | "%=") =>
            {
                Some(op.as_str())
            }
            _ => None,
        }
    }

    fn is_assignment_operator(&self) -> bool {
        self.assignment_operator().is_some()
    }

    fn is_simple_assignment_operator(&self) -> bool {
        matches!(self.peek(), TokenKind::Operator(op) if op == ":=" || op == "=")
    }

    fn consume_assignment_operator(&mut self, context: &str) -> bool {
        if self.is_simple_assignment_operator() {
            self.advance();
            true
        } else {
            let found = format!("{:?}", self.peek());
            self.push_diagnostic(format!(
                "Expected assignment operator (':=' or '=') {} but found {}",
                context, found
            ));
            false
        }
    }

    fn consume_statement_assignment_operator(&mut self) -> Option<String> {
        let operator = self.assignment_operator()?.to_string();
        self.advance();
        Some(operator)
    }

    fn compound_assignment_binary_operator(operator: &str) -> Option<&'static str> {
        match operator {
            "+=" => Some("+"),
            "-=" => Some("-"),
            "*=" => Some("*"),
            "/=" => Some("/"),
            "%=" => Some("%"),
            _ => None,
        }
    }

    fn expect_keyword(&mut self, expected: &str, context: &str) -> bool {
        if matches!(self.peek(), TokenKind::Keyword(found) if found == expected) {
            self.advance();
            true
        } else {
            let found = format!("{:?}", self.peek());
            self.push_diagnostic(format!(
                "Expected '{}' {} but found {}",
                expected, context, found
            ));
            false
        }
    }

    fn synchronize_statement(&mut self) {
        while !matches!(self.peek(), TokenKind::Eof) {
            if matches!(self.peek(), TokenKind::Punctuation(';')) {
                self.advance();
                return;
            }

            if matches!(self.peek(), TokenKind::Punctuation('}')) {
                return;
            }

            if matches!(
                self.peek(),
                TokenKind::Identifier(_)
                    | TokenKind::Int(_)
                    | TokenKind::Float(_)
                    | TokenKind::String(_)
                    | TokenKind::Bool(_)
                    | TokenKind::InterpolatedString(_)
                    | TokenKind::Punctuation('[')
                    | TokenKind::Punctuation('{')
                    | TokenKind::Punctuation('(')
            ) {
                return;
            }

            if matches!(self.peek(), TokenKind::Keyword(keyword) if matches!(
                keyword.as_str(),
                "let"
                    | "mut"
                    | "const"
                    | "func"
                    | "async"
                    | "if"
                    | "for"
                    | "while"
                    | "loop"
                    | "match"
                    | "return"
                    | "break"
                    | "continue"
                    | "try"
                    | "test"
                    | "test_setup"
                    | "test_teardown"
                    | "test_group"
                    | "import"
                    | "from"
                    | "export"
            )) {
                return;
            }

            self.advance();
        }
    }

    fn is_valid_assignment_target(expr: &Expr) -> bool {
        matches!(expr, Expr::Identifier(_) | Expr::FieldAccess { .. } | Expr::IndexAccess { .. })
    }

    /// Get the source location of the current token
    /// Used in Phase 2 for capturing AST node locations
    #[allow(dead_code)]
    fn current_location(&self) -> SourceLocation {
        if let Some(token) = self.tokens.get(self.pos) {
            SourceLocation::new(token.line, token.column)
        } else {
            SourceLocation::unknown()
        }
    }

    /// Get the source location at a specific position in the token stream
    /// Used in Phase 2 for capturing AST node locations
    #[allow(dead_code)]
    fn location_at(&self, pos: usize) -> SourceLocation {
        if let Some(token) = self.tokens.get(pos) {
            SourceLocation::new(token.line, token.column)
        } else {
            SourceLocation::unknown()
        }
    }

    /// Parse the entire token stream into statements and parser diagnostics.
    pub fn parse_with_diagnostics(&mut self) -> ParseOutput {
        self.diagnostics.clear();
        self.ast_spans.clear();
        let mut stmts = Vec::new();
        while !matches!(self.peek(), TokenKind::Eof) {
            // Skip semicolons between statements
            if matches!(self.peek(), TokenKind::Punctuation(';')) {
                self.advance();
                continue;
            }

            let diagnostics_before = self.diagnostics.len();
            let pos_before = self.pos;
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                if self.diagnostics.len() == diagnostics_before {
                    self.push_diagnostic("Invalid statement");
                }
                if self.pos == pos_before {
                    self.advance();
                }
                self.synchronize_statement();
            }
        }
        ParseOutput {
            stmts,
            diagnostics: std::mem::take(&mut self.diagnostics),
            ast_spans: std::mem::take(&mut self.ast_spans),
        }
    }

    /// Parse the entire token stream and return statements only when no parse diagnostics exist.
    pub fn parse(&mut self) -> Vec<Stmt> {
        let output = self.parse_with_diagnostics();
        if output.diagnostics.is_empty() {
            output.stmts
        } else {
            Vec::new()
        }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let start_pos = self.pos;
        let parsed = self.parse_stmt_inner();
        if parsed.is_some() {
            self.record_ast_span(AstNodeSpanKind::Statement, start_pos, self.pos);
        }
        parsed
    }

    fn parse_stmt_inner(&mut self) -> Option<Stmt> {
        match self.peek() {
            TokenKind::Keyword(k) if k == "let" || k == "mut" => self.parse_let(),
            TokenKind::Keyword(k) if k == "const" => self.parse_const(),
            TokenKind::Keyword(k) if k == "async" => {
                // async func or async function expression
                self.advance(); // consume 'async'
                if matches!(self.peek(), TokenKind::Keyword(k) if k == "func") {
                    self.parse_func_with_async(true)
                } else {
                    self.push_diagnostic("Expected 'func' after 'async'");
                    None
                }
            }
            TokenKind::Keyword(k) if k == "func" => self.parse_func_with_async(false),
            TokenKind::Keyword(k) if k == "enum" => self.parse_enum(),
            TokenKind::Keyword(k) if k == "struct" => self.parse_struct(),
            TokenKind::Keyword(k) if k == "import" || k == "from" => self.parse_import(),
            TokenKind::Keyword(k) if k == "export" => self.parse_export(),
            TokenKind::Keyword(k) if k == "return" => {
                self.advance();
                let expr = if !matches!(
                    self.peek(),
                    TokenKind::Punctuation(';') | TokenKind::Punctuation('}') | TokenKind::Eof
                ) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                Some(Stmt::Return(expr))
            }
            TokenKind::Keyword(k) if k == "if" => self.parse_if(),
            TokenKind::Keyword(k) if k == "try" => self.parse_try_except(),
            TokenKind::Keyword(k) if k == "match" => self.parse_match(),
            TokenKind::Keyword(k) if k == "loop" => self.parse_loop(),
            TokenKind::Keyword(k) if k == "while" => self.parse_while(),
            TokenKind::Keyword(k) if k == "for" => self.parse_for(),
            TokenKind::Keyword(k) if k == "spawn" => self.parse_spawn(),
            TokenKind::Keyword(k) if k == "test" => self.parse_test(),
            TokenKind::Keyword(k) if k == "test_setup" => self.parse_test_setup(),
            TokenKind::Keyword(k) if k == "test_teardown" => self.parse_test_teardown(),
            TokenKind::Keyword(k) if k == "test_group" => self.parse_test_group(),
            TokenKind::Keyword(k) if k == "break" => {
                self.advance();
                Some(Stmt::Break)
            }
            TokenKind::Keyword(k) if k == "continue" => {
                self.advance();
                Some(Stmt::Continue)
            }
            // Handle destructuring patterns: [a, b] := expr or {x, y} := expr
            TokenKind::Punctuation('[') | TokenKind::Punctuation('{') => {
                let saved_pos = self.pos;
                // Try to parse as destructuring pattern
                if let Some(pattern) = self.parse_pattern() {
                    // Check if next token is :=
                    if self.is_simple_assignment_operator() {
                        self.advance(); // consume :=
                        let value = self.parse_expr()?;
                        return Some(Stmt::Let {
                            pattern,
                            value,
                            mutable: false,
                            type_annotation: None,
                        });
                    }
                }
                // Not a destructuring pattern, restore and parse as expression
                self.pos = saved_pos;
                self.parse_expr().map(Stmt::ExprStmt)
            }
            TokenKind::Identifier(_) => {
                // Check for variable assignment (name := expr or expr[...] := expr)
                // We need to look ahead and parse an expression to see if it's followed by :=
                let saved_pos = self.pos;
                if let Some(expr) = self.parse_expr() {
                    // Check if next token is an assignment operator.
                    if let Some(operator) = self.consume_statement_assignment_operator() {
                        let target = expr;
                        if !Self::is_valid_assignment_target(&target) {
                            self.push_diagnostic("Invalid assignment target");
                            return None;
                        }

                        let rhs = self.parse_expr()?;
                        if self.is_assignment_operator() {
                            self.push_diagnostic(
                                "Chained assignment is not supported; split the assignment into separate statements",
                            );
                            return None;
                        }

                        // Lower compound assignments into regular assignment + binary operation.
                        // Example: `x += y` -> `x := x + y`
                        let value = if let Some(binary_op) =
                            Self::compound_assignment_binary_operator(operator.as_str())
                        {
                            Expr::BinaryOp {
                                left: Box::new(target.clone()),
                                op: binary_op.to_string(),
                                right: Box::new(rhs),
                            }
                        } else {
                            rhs
                        };

                        Some(Stmt::Assign { target, value })
                    } else {
                        // Not an assignment, restore position and parse as expression statement.
                        self.pos = saved_pos;
                        self.parse_expr().map(Stmt::ExprStmt)
                    }
                } else {
                    None
                }
            }
            _ => self.parse_expr().map(Stmt::ExprStmt),
        }
    }

    fn parse_enum(&mut self) -> Option<Stmt> {
        self.advance(); // enum
        let name = match self.advance() {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                self.push_diagnostic("Expected enum name after 'enum'");
                return None;
            }
        };
        if !self.expect_punctuation('{', "to start enum body") {
            return None;
        }
        let mut variants = Vec::new();
        while let TokenKind::Identifier(v) = self.peek() {
            variants.push(v.clone());
            self.advance();
            if matches!(self.peek(), TokenKind::Punctuation(',')) {
                self.advance();
            } else {
                break;
            }
        }
        if !self.expect_punctuation('}', "to close enum body") {
            return None;
        }
        Some(Stmt::EnumDef { name, variants })
    }

    fn parse_struct(&mut self) -> Option<Stmt> {
        self.advance(); // struct
        let name = match self.advance() {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                self.push_diagnostic("Expected struct name after 'struct'");
                return None;
            }
        };

        if !self.expect_punctuation('{', "to start struct body") {
            return None;
        }
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !matches!(self.peek(), TokenKind::Punctuation('}'))
            && !matches!(self.peek(), TokenKind::Eof)
        {
            // Check if this is a method definition (async func or func)
            let is_async = if matches!(self.peek(), TokenKind::Keyword(k) if k == "async") {
                self.advance(); // consume 'async'
                true
            } else {
                false
            };

            if matches!(self.peek(), TokenKind::Keyword(k) if k == "func") {
                if let Some(method) = self.parse_func_with_async(is_async) {
                    methods.push(method);
                }
            } else if let TokenKind::Identifier(field_name) = self.peek() {
                // Parse field: name: type
                let field_name = field_name.clone();
                self.advance();

                let field_type = self.parse_type_annotation();
                fields.push((field_name, field_type));

                // Consume optional comma
                if matches!(self.peek(), TokenKind::Punctuation(',')) {
                    self.advance();
                }
                // If no comma, continue to next field or closing brace
            } else {
                // Unknown token, skip it to avoid infinite loop
                self.advance();
            }
        }

        if !self.expect_punctuation('}', "to close struct body") {
            return None;
        }
        Some(Stmt::StructDef { name, fields, methods })
    }

    fn parse_let(&mut self) -> Option<Stmt> {
        // Handle 'mut', 'let', or bare identifier with :=
        let is_mut = match self.peek() {
            TokenKind::Keyword(k) if k == "mut" => {
                self.advance();
                true
            }
            TokenKind::Keyword(k) if k == "let" => {
                self.advance();
                false
            }
            _ => false, // Plain identifier (e.g., val := ...)
        };

        // Parse pattern (identifier, array destructuring, or dict destructuring)
        let pattern = self.parse_pattern()?;

        // Parse optional type annotation (: type)
        let type_annotation = self.parse_type_annotation();

        if !self.consume_assignment_operator("in let declaration") {
            return None;
        }
        let value = self.parse_expr()?;
        Some(Stmt::Let { pattern, value, mutable: is_mut, type_annotation })
    }

    /// Parse a destructuring pattern
    fn parse_pattern(&mut self) -> Option<crate::ast::Pattern> {
        use crate::ast::Pattern;

        match self.peek() {
            // Array destructuring: [a, b, ...rest]
            TokenKind::Punctuation('[') => {
                self.advance(); // [
                let mut elements = Vec::new();
                let mut rest = None;

                loop {
                    match self.peek() {
                        TokenKind::Punctuation(']') => {
                            self.advance();
                            break;
                        }
                        TokenKind::Operator(op) if op == "..." => {
                            // Rest element: ...rest
                            self.advance(); // ...
                            if let TokenKind::Identifier(name) = self.advance() {
                                rest = Some(name.clone());
                            }
                            // After rest element, expect closing bracket
                            if matches!(self.peek(), TokenKind::Punctuation(']')) {
                                self.advance();
                                break;
                            }
                        }
                        TokenKind::Identifier(name) if name == "_" => {
                            // Ignore placeholder
                            self.advance();
                            elements.push(Pattern::Ignore);
                        }
                        _ => {
                            // Regular pattern (can be nested)
                            let pattern = self.parse_pattern()?;
                            elements.push(pattern);
                        }
                    }

                    // Check for comma or closing bracket
                    match self.peek() {
                        TokenKind::Punctuation(',') => {
                            self.advance();
                        }
                        TokenKind::Punctuation(']') => {
                            self.advance();
                            break;
                        }
                        _ => break,
                    }
                }

                Some(Pattern::Array { elements, rest })
            }
            // Dict destructuring: {name, email, ...rest}
            TokenKind::Punctuation('{') => {
                self.advance(); // {
                let mut keys = Vec::new();
                let mut rest = None;

                loop {
                    match self.peek() {
                        TokenKind::Punctuation('}') => {
                            self.advance();
                            break;
                        }
                        TokenKind::Operator(op) if op == "..." => {
                            // Rest element: ...rest
                            self.advance(); // ...
                            if let TokenKind::Identifier(name) = self.advance() {
                                rest = Some(name.clone());
                            }
                            // After rest element, expect closing brace
                            if matches!(self.peek(), TokenKind::Punctuation('}')) {
                                self.advance();
                                break;
                            }
                        }
                        TokenKind::Identifier(key) => {
                            keys.push(key.clone());
                            self.advance();
                        }
                        _ => break,
                    }

                    // Check for comma or closing brace
                    match self.peek() {
                        TokenKind::Punctuation(',') => {
                            self.advance();
                        }
                        TokenKind::Punctuation('}') => {
                            self.advance();
                            break;
                        }
                        _ => break,
                    }
                }

                Some(Pattern::Dict { keys, rest })
            }
            // Ignore placeholder: _
            TokenKind::Identifier(name) if name == "_" => {
                self.advance();
                Some(Pattern::Ignore)
            }
            // Simple identifier pattern
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Some(Pattern::Identifier(name))
            }
            _ => None,
        }
    }

    fn parse_const(&mut self) -> Option<Stmt> {
        self.advance(); // const
        let name = match self.advance() {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                self.push_diagnostic("Expected constant name after 'const'");
                return None;
            }
        };

        // Parse optional type annotation (: type)
        let type_annotation = self.parse_type_annotation();

        if !self.consume_assignment_operator("in const declaration") {
            return None;
        }
        let value = self.parse_expr()?;
        Some(Stmt::Const { name, value, type_annotation })
    }

    fn parse_func_with_async(&mut self, is_async: bool) -> Option<Stmt> {
        self.advance(); // func

        // Check for generator syntax: func*
        let is_generator = if matches!(self.peek(), TokenKind::Operator(op) if op == "*") {
            self.advance(); // consume *
            true
        } else {
            false
        };

        let name = match self.advance() {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                self.push_diagnostic("Expected function name after 'func'");
                return None;
            }
        };
        if !self.expect_punctuation('(', "after function name") {
            return None;
        }
        let mut params = Vec::new();
        let mut param_types = Vec::new();

        // Parse parameters - handle both identifiers and 'self' keyword
        loop {
            match self.peek() {
                TokenKind::Identifier(p) => {
                    params.push(p.clone());
                    self.advance();
                }
                TokenKind::Keyword(k) if k == "self" => {
                    params.push("self".to_string());
                    self.advance();
                }
                _ => break, // No more parameters
            }

            // Parse optional type annotation for parameter
            let param_type = self.parse_type_annotation();
            param_types.push(param_type);

            if matches!(self.peek(), TokenKind::Punctuation(',')) {
                self.advance();
            } else {
                break;
            }
        }
        if !self.expect_punctuation(')', "to close function parameter list") {
            return None;
        }

        // Parse optional return type annotation (-> type)
        let return_type = if matches!(self.peek(), TokenKind::Operator(op) if op == "->") {
            self.advance(); // ->
            match self.peek() {
                TokenKind::Keyword(k) if k == "int" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Int)
                }
                TokenKind::Keyword(k) if k == "float" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Float)
                }
                TokenKind::Keyword(k) if k == "string" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::String)
                }
                TokenKind::Keyword(k) if k == "bool" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Bool)
                }
                _ => None,
            }
        } else {
            None
        };

        let body = self.parse_statement_block(
            "to start function body",
            "to close function body",
            "function body",
        )?;
        Some(Stmt::FuncDef { name, param_types, return_type, params, body, is_generator, is_async })
    }

    /// Parse a function expression (anonymous function)
    fn parse_func_expr_with_async(&mut self, is_async: bool) -> Option<Expr> {
        self.advance(); // func

        // Check for generator syntax: func*
        let is_generator = if matches!(self.peek(), TokenKind::Operator(op) if op == "*") {
            self.advance(); // consume *
            true
        } else {
            false
        };

        if !self.expect_punctuation('(', "after 'func' in function expression") {
            return None;
        }
        let mut params = Vec::new();
        let mut param_types = Vec::new();

        while let TokenKind::Identifier(p) = self.peek() {
            params.push(p.clone());
            self.advance();

            // Parse optional type annotation for parameter
            let param_type = self.parse_type_annotation();
            param_types.push(param_type);

            if matches!(self.peek(), TokenKind::Punctuation(',')) {
                self.advance();
            } else {
                break;
            }
        }
        if !self.expect_punctuation(')', "to close function expression parameter list") {
            return None;
        }

        // Parse optional return type annotation (-> type)
        let return_type = if matches!(self.peek(), TokenKind::Operator(op) if op == "->") {
            self.advance(); // ->
            match self.peek() {
                TokenKind::Keyword(k) if k == "int" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Int)
                }
                TokenKind::Keyword(k) if k == "float" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Float)
                }
                TokenKind::Keyword(k) if k == "string" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::String)
                }
                TokenKind::Keyword(k) if k == "bool" => {
                    self.advance();
                    Some(crate::ast::TypeAnnotation::Bool)
                }
                _ => None,
            }
        } else {
            None
        };

        let body = self.parse_statement_block(
            "to start function expression body",
            "to close function expression body",
            "function expression body",
        )?;
        Some(Expr::Function { params, param_types, return_type, body, is_generator, is_async })
    }

    fn parse_match(&mut self) -> Option<Stmt> {
        self.advance(); // match
        let value = self.parse_expr()?;
        self.advance(); // {
        let mut cases = Vec::new();
        let mut default = None;

        while !matches!(self.peek(), TokenKind::Punctuation('}')) {
            match self.peek() {
                TokenKind::Keyword(k) if k == "case" => {
                    self.advance(); // case

                    // Parse pattern which might be Base::Variant or Base::Variant(var)
                    let pat = match self.advance() {
                        TokenKind::Identifier(s) => s.clone(),
                        _tok => {
                            return None;
                        }
                    };

                    // Check for :: operator (enum variant)
                    let pat_str = if matches!(self.peek(), TokenKind::Operator(op) if op == "::") {
                        self.advance(); // ::
                        let variant = match self.advance() {
                            TokenKind::Identifier(v) => v.clone(),
                            _ => return None,
                        };
                        let full_tag = format!("{}::{}", pat, variant);

                        // Check for parameter binding like Result::Ok(msg)
                        if matches!(self.peek(), TokenKind::Punctuation('(')) {
                            self.advance(); // (
                            let var = match self.advance() {
                                TokenKind::Identifier(v) => v.clone(),
                                _ => return None,
                            };
                            self.advance(); // )
                            format!("{}({})", full_tag, var)
                        } else {
                            full_tag
                        }
                    } else {
                        // Plain identifier pattern
                        if matches!(self.peek(), TokenKind::Punctuation('(')) {
                            self.advance(); // (
                            let var = match self.advance() {
                                TokenKind::Identifier(v) => v.clone(),
                                _ => return None,
                            };
                            self.advance(); // )
                            format!("{}({})", pat, var)
                        } else {
                            pat
                        }
                    };

                    self.advance(); // :
                    let body = self.parse_statement_block(
                        "to start match case block",
                        "to close match case block",
                        "match case block",
                    )?;
                    cases.push((pat_str, body));
                }
                TokenKind::Keyword(k) if k == "default" => {
                    self.advance(); // default
                    self.advance(); // :
                    default = Some(self.parse_statement_block(
                        "to start match default block",
                        "to close match default block",
                        "match default block",
                    )?);
                }
                _ => break,
            }
        }

        self.advance(); // }
        Some(Stmt::Match { value, cases, default })
    }

    fn parse_loop(&mut self) -> Option<Stmt> {
        self.advance(); // loop
        let condition = if matches!(self.peek(), TokenKind::Keyword(k) if k == "while") {
            self.advance(); // while
            Some(self.parse_expr()?)
        } else {
            None
        };
        let body =
            self.parse_statement_block("to start loop body", "to close loop body", "loop body")?;
        Some(Stmt::Loop { condition, body })
    }

    fn parse_while(&mut self) -> Option<Stmt> {
        self.advance(); // while
        let condition = self.parse_expr()?;
        let body =
            self.parse_statement_block("to start while body", "to close while body", "while body")?;
        Some(Stmt::While { condition, body })
    }

    fn parse_for(&mut self) -> Option<Stmt> {
        self.advance(); // for
        let var = match self.advance() {
            TokenKind::Identifier(v) => v.clone(),
            _ => {
                self.push_diagnostic("Expected loop variable name after 'for'");
                return None;
            }
        };
        if !self.expect_keyword("in", "in for loop") {
            return None;
        }
        // Use parse_call to parse the iterable expression
        // This allows function calls like: for x in generator_func() { ... }
        // but avoids struct instantiation syntax
        let iterable = self.parse_call()?;
        let body = self.parse_statement_block(
            "to start for loop body",
            "to close for loop body",
            "for loop body",
        )?;
        Some(Stmt::For { var, iterable, body })
    }

    fn parse_spawn(&mut self) -> Option<Stmt> {
        self.advance(); // spawn
        let body = self.parse_statement_block(
            "to start spawn block",
            "to close spawn block",
            "spawn block",
        )?;
        Some(Stmt::Spawn { body })
    }

    fn parse_test(&mut self) -> Option<Stmt> {
        self.advance(); // test
                        // Expect string literal for test name
        let name = match self.advance() {
            TokenKind::String(s) => s.clone(),
            _ => {
                self.push_diagnostic("Expected test name string after 'test'");
                return None;
            }
        };
        let body =
            self.parse_statement_block("to start test body", "to close test body", "test body")?;
        Some(Stmt::Test { name, body })
    }

    fn parse_test_setup(&mut self) -> Option<Stmt> {
        self.advance(); // test_setup
        let body = self.parse_statement_block(
            "to start test_setup block",
            "to close test_setup block",
            "test_setup block",
        )?;
        Some(Stmt::TestSetup { body })
    }

    fn parse_test_teardown(&mut self) -> Option<Stmt> {
        self.advance(); // test_teardown
        let body = self.parse_statement_block(
            "to start test_teardown block",
            "to close test_teardown block",
            "test_teardown block",
        )?;
        Some(Stmt::TestTeardown { body })
    }

    fn parse_test_group(&mut self) -> Option<Stmt> {
        self.advance(); // test_group
                        // Expect string literal for group name
        let name = match self.advance() {
            TokenKind::String(s) => s.clone(),
            _ => {
                self.push_diagnostic("Expected group name string after 'test_group'");
                return None;
            }
        };
        let tests = self.parse_statement_block(
            "to start test_group body",
            "to close test_group body",
            "test_group body",
        )?;
        Some(Stmt::TestGroup { name, tests })
    }

    fn parse_try_except(&mut self) -> Option<Stmt> {
        self.advance(); // try
        let try_block =
            self.parse_statement_block("to start try block", "to close try block", "try block")?;
        if !self.expect_keyword("except", "after try block") {
            return None;
        }
        let except_var = match self.advance() {
            TokenKind::Identifier(v) => v.clone(),
            _ => {
                self.push_diagnostic("Expected exception variable after 'except'");
                return None;
            }
        };
        let except_block = self.parse_statement_block(
            "to start except block",
            "to close except block",
            "except block",
        )?;
        Some(Stmt::TryExcept { try_block, except_var, except_block })
    }

    fn parse_import(&mut self) -> Option<Stmt> {
        // Two forms:
        // 1. import module
        // 2. from module import symbol1, symbol2

        let is_from = matches!(self.peek(), TokenKind::Keyword(k) if k == "from");
        self.advance(); // import or from

        if is_from {
            // from module import ...
            let module = self.parse_from_import_module_path()?;

            // expect 'import' keyword
            if !self.expect_keyword("import", "after module name in from-import statement") {
                return None;
            }

            // Parse symbol list
            let mut symbols = Vec::new();
            loop {
                match self.advance() {
                    TokenKind::Identifier(s) => symbols.push(s.clone()),
                    _ => {
                        self.push_diagnostic("Expected imported symbol name");
                        return None;
                    }
                }

                if matches!(self.peek(), TokenKind::Punctuation(',')) {
                    self.advance(); // ,
                } else {
                    break;
                }
            }

            Some(Stmt::Import { module, symbols: Some(symbols) })
        } else {
            // import module
            let module = match self.advance() {
                TokenKind::Identifier(m) => m.clone(),
                _ => {
                    self.push_diagnostic("Expected module name after 'import'");
                    return None;
                }
            };

            Some(Stmt::Import { module, symbols: None })
        }
    }

    fn parse_from_import_module_path(&mut self) -> Option<String> {
        let mut module = match self.advance() {
            TokenKind::Identifier(m) => m.clone(),
            _ => {
                self.push_diagnostic("Expected module name after 'from'");
                return None;
            }
        };

        while matches!(self.peek(), TokenKind::Punctuation('.')) {
            self.advance(); // .
            match self.advance() {
                TokenKind::Identifier(segment) => {
                    module.push('.');
                    module.push_str(segment);
                }
                _ => {
                    self.push_diagnostic(
                        "Expected module path segment after '.' in from-import statement",
                    );
                    return None;
                }
            }
        }

        Some(module)
    }

    fn parse_export(&mut self) -> Option<Stmt> {
        self.advance(); // export

        // Parse the statement to be exported
        let stmt = self.parse_stmt()?;

        Some(Stmt::Export { stmt: Box::new(stmt) })
    }

    fn parse_if(&mut self) -> Option<Stmt> {
        self.advance(); // if
        let condition = self.parse_expr()?;
        let then_branch =
            self.parse_statement_block("to start if block", "to close if block", "if block")?;

        let else_branch = if matches!(self.peek(), TokenKind::Keyword(k) if k == "else") {
            self.advance(); // else
            if matches!(self.peek(), TokenKind::Keyword(k) if k == "if") {
                let nested_if = self.parse_if()?;
                Some(vec![nested_if])
            } else {
                self.parse_statement_block(
                    "to start else block",
                    "to close else block",
                    "else block",
                )
            }
        } else {
            None
        };

        Some(Stmt::If { condition, then_branch, else_branch })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        let start_pos = self.pos;
        let parsed = self.with_expression_depth("expression", |parser| parser.parse_expr_inner());
        if parsed.is_some() {
            self.record_ast_span(AstNodeSpanKind::Expression, start_pos, self.pos);
        }
        parsed
    }

    fn parse_expr_inner(&mut self) -> Option<Expr> {
        // Check for enum tag (e.g., Result::Ok(...))
        if let TokenKind::Identifier(a) = self.peek() {
            if self.tokens.get(self.pos + 1).map(|t| &t.kind)
                == Some(&TokenKind::Operator("::".into()))
            {
                let base = a.clone();
                self.advance(); // base
                self.advance(); // ::
                let variant = match self.advance() {
                    TokenKind::Identifier(v) => v.clone(),
                    _ => return None,
                };
                let mut args = Vec::new();
                if matches!(self.peek(), TokenKind::Punctuation('(')) {
                    self.advance();
                    while !matches!(self.peek(), TokenKind::Punctuation(')'))
                        && !matches!(self.peek(), TokenKind::Eof)
                    {
                        if let Some(arg) = self.parse_expr() {
                            args.push(arg);
                        }
                        if matches!(self.peek(), TokenKind::Punctuation(',')) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if !self.expect_punctuation(')', "to close tag expression arguments") {
                        return None;
                    }
                }
                return Some(Expr::Tag(format!("{}::{}", base, variant), args));
            }
        }

        // Check for throw - still uses Tag since it's a control-flow primitive
        if let TokenKind::Identifier(name) = self.peek() {
            let name_clone = name.clone();
            if name_clone.as_str() == "throw"
                && self.tokens.get(self.pos + 1).map(|t| &t.kind)
                    == Some(&TokenKind::Punctuation('('))
            {
                self.advance(); // name
                self.advance(); // (
                let mut args = Vec::new();
                while !matches!(self.peek(), TokenKind::Punctuation(')'))
                    && !matches!(self.peek(), TokenKind::Eof)
                {
                    if let Some(arg) = self.parse_expr() {
                        args.push(arg);
                    }
                    if matches!(self.peek(), TokenKind::Punctuation(',')) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if !self.expect_punctuation(')', "to close throw(...) arguments") {
                    return None;
                }
                return Some(Expr::Tag(name_clone, args));
            }
        }

        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> Option<Expr> {
        let mut left = self.parse_null_coalescing()?;

        while matches!(self.peek(), TokenKind::Operator(op) if op == "|>") {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_null_coalescing()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_null_coalescing(&mut self) -> Option<Expr> {
        let mut left = self.parse_or()?;

        while matches!(self.peek(), TokenKind::Operator(op) if op == "??") {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_or()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;

        while matches!(self.peek(), TokenKind::Operator(op) if op == "||") {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_and()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_equality()?;

        while matches!(self.peek(), TokenKind::Operator(op) if op == "&&") {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_equality()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let mut left = self.parse_comparison()?;

        while matches!(self.peek(), TokenKind::Operator(op) if matches!(op.as_str(), "==" | "!=")) {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let mut left = self.parse_additive()?;

        while matches!(
            self.peek(),
            TokenKind::Operator(op) if matches!(op.as_str(), ">" | "<" | ">=" | "<=")
        ) {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_additive()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        let mut left = self.parse_multiplicative()?;

        while matches!(self.peek(), TokenKind::Operator(op) if matches!(op.as_str(), "+" | "-")) {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;

        while matches!(self.peek(), TokenKind::Operator(op) if matches!(op.as_str(), "*" | "/" | "%"))
        {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => break,
            };
            let right = self.parse_unary()?;
            left = Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }

        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        // Check for unary operators: - and !
        if matches!(self.peek(), TokenKind::Operator(op) if matches!(op.as_str(), "-" | "!")) {
            let op = match self.advance() {
                TokenKind::Operator(o) => o.clone(),
                _ => return None,
            };
            let operand =
                self.with_expression_depth("unary expression", |parser| parser.parse_unary())?; // Recursive for nested unary ops like --x
            return Some(Expr::UnaryOp { op, operand: Box::new(operand) });
        }

        // If not a unary operator, parse as call/postfix expression
        self.parse_call()
    }

    fn parse_call(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                // Handle function calls
                TokenKind::Punctuation('(') => {
                    self.advance(); // (
                    let mut args = Vec::new();
                    while !matches!(self.peek(), TokenKind::Punctuation(')'))
                        && !matches!(self.peek(), TokenKind::Eof)
                    {
                        if let Some(arg) = self.parse_expr() {
                            args.push(arg);
                        }
                        if matches!(self.peek(), TokenKind::Punctuation(',')) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if !self.expect_punctuation(')', "to close function call arguments") {
                        return None;
                    }
                    expr = Expr::Call { function: Box::new(expr), args };
                }
                // Handle field access and method calls
                TokenKind::Punctuation('.') => {
                    self.advance(); // .
                    if let TokenKind::Identifier(field) = self.peek() {
                        let field_name = field.clone();
                        self.advance();

                        // Check if this is a method call (field access followed by ())
                        if matches!(self.peek(), TokenKind::Punctuation('(')) {
                            self.advance(); // (
                            let mut args = Vec::new();
                            while !matches!(self.peek(), TokenKind::Punctuation(')'))
                                && !matches!(self.peek(), TokenKind::Eof)
                            {
                                if let Some(arg) = self.parse_expr() {
                                    args.push(arg);
                                }
                                if matches!(self.peek(), TokenKind::Punctuation(',')) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            if !self.expect_punctuation(')', "to close method call arguments") {
                                return None;
                            }
                            expr = Expr::MethodCall {
                                object: Box::new(expr),
                                method: field_name,
                                args,
                            };
                        } else {
                            // Just a field access
                            expr = Expr::FieldAccess { object: Box::new(expr), field: field_name };
                        }
                    } else {
                        break;
                    }
                }
                // Handle optional chaining: obj?.field
                TokenKind::Operator(op) if op == "?." => {
                    self.advance(); // ?.
                    if let TokenKind::Identifier(field) = self.peek() {
                        let field_name = field.clone();
                        self.advance();
                        // Optional chaining returns null if object is null, otherwise accesses field
                        // We'll represent this as a BinaryOp with special handling in the interpreter
                        expr = Expr::BinaryOp {
                            left: Box::new(expr),
                            op: "?.".to_string(),
                            right: Box::new(Expr::String(field_name)),
                        };
                    } else {
                        break;
                    }
                }
                // Handle try operator: expr?
                TokenKind::Operator(op) if op == "?" => {
                    self.advance(); // ?
                    expr = Expr::Try(Box::new(expr));
                }
                // Handle index access: arr[index]
                TokenKind::Punctuation('[') => {
                    self.advance(); // [
                    let index = self.parse_expr()?;
                    if !self.expect_punctuation(']', "to close index expression") {
                        return None;
                    }
                    expr = Expr::IndexAccess { object: Box::new(expr), index: Box::new(index) };
                }
                // Handle struct instantiation: Struct { field1: val1, field2: val2 }
                TokenKind::Punctuation('{') if matches!(expr, Expr::Identifier(_)) => {
                    // Only treat as struct instantiation if we have an identifier followed by {
                    // AND there's actually struct field syntax inside (field: value)
                    // Check if next token looks like a field (identifier followed by colon or closing brace)
                    let next_token = self.tokens.get(self.pos + 1);
                    let is_struct = match next_token.map(|t| &t.kind) {
                        Some(TokenKind::Identifier(_)) => {
                            // Check if there's a colon after the identifier
                            self.tokens.get(self.pos + 2).map(|t| &t.kind)
                                == Some(&TokenKind::Punctuation(':'))
                        }
                        Some(TokenKind::Punctuation('}')) => {
                            // Empty braces {} - treat as empty struct
                            true
                        }
                        _ => false,
                    };

                    if !is_struct {
                        // Not a struct instantiation, stop parsing here
                        break;
                    }

                    // This is a struct instantiation
                    if let Expr::Identifier(name) = expr {
                        self.advance(); // {
                        let mut fields = Vec::new();

                        loop {
                            if matches!(self.peek(), TokenKind::Punctuation('}')) {
                                break;
                            }

                            if let TokenKind::Identifier(field_name) = self.peek() {
                                let field_name = field_name.clone();
                                self.advance(); // consume field name

                                // Expect colon
                                if !matches!(self.peek(), TokenKind::Punctuation(':')) {
                                    break; // Invalid syntax
                                }
                                self.advance(); // consume :

                                // Parse field value - use parse_equality to avoid infinite recursion
                                // while still supporting expressions like x + y, x * 2, etc.
                                if let Some(value) = self.parse_equality() {
                                    fields.push((field_name, value));
                                }

                                // Check for comma or end
                                if matches!(self.peek(), TokenKind::Punctuation(',')) {
                                    self.advance();
                                } else if matches!(self.peek(), TokenKind::Punctuation('}')) {
                                    break;
                                } else {
                                    // Unexpected token, try to recover
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        if !self.expect_punctuation('}', "to close struct literal") {
                            return None;
                        }
                        expr = Expr::StructInstance { name, fields };
                    }
                    break;
                }
                _ => break,
            }
        }

        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.peek() {
            TokenKind::Punctuation('[') => self.parse_array_literal(),
            TokenKind::Punctuation('{') => self.parse_dict_literal(),
            TokenKind::Keyword(k) if k == "async" => {
                // async function expression
                self.advance(); // consume 'async'
                if matches!(self.peek(), TokenKind::Keyword(k) if k == "func") {
                    self.parse_func_expr_with_async(true)
                } else {
                    self.push_diagnostic("Expected 'func' after 'async'");
                    None
                }
            }
            TokenKind::Keyword(k) if k == "func" => self.parse_func_expr_with_async(false),
            TokenKind::Keyword(k) if k == "yield" => {
                self.advance(); // consume yield
                                // yield can have an optional value
                let value = if !matches!(
                    self.peek(),
                    TokenKind::Punctuation(';') | TokenKind::Punctuation('}')
                ) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                Some(Expr::Yield(value))
            }
            TokenKind::Keyword(k) if k == "await" => {
                self.advance(); // consume await
                                // await requires an expression (the promise to wait for)
                let promise_expr = Box::new(self.parse_expr()?);
                Some(Expr::Await(promise_expr))
            }
            TokenKind::Keyword(k) if k == "null" => {
                self.advance();
                Some(Expr::Identifier("null".to_string()))
            }
            TokenKind::Keyword(k) if k == "self" => {
                // Treat 'self' as an identifier in expression context
                self.advance();
                Some(Expr::Identifier("self".to_string()))
            }
            TokenKind::Identifier(id) if id == "None" => {
                // Handle None (no arguments)
                self.advance();
                Some(Expr::None)
            }
            TokenKind::Identifier(id) if id == "Ok" => {
                // Handle Ok(value)
                self.advance(); // consume Ok
                if matches!(self.peek(), TokenKind::Punctuation('(')) {
                    self.advance(); // consume (
                    let value = self.parse_expr()?;
                    if !self.expect_punctuation(')', "to close Ok(...)") {
                        return None;
                    }
                    Some(Expr::Ok(Box::new(value)))
                } else {
                    self.push_diagnostic("Expected '(' after 'Ok'");
                    None
                }
            }
            TokenKind::Identifier(id) if id == "Err" => {
                // Handle Err(error)
                self.advance(); // consume Err
                if matches!(self.peek(), TokenKind::Punctuation('(')) {
                    self.advance(); // consume (
                    let error = self.parse_expr()?;
                    if !self.expect_punctuation(')', "to close Err(...)") {
                        return None;
                    }
                    Some(Expr::Err(Box::new(error)))
                } else {
                    self.push_diagnostic("Expected '(' after 'Err'");
                    None
                }
            }
            TokenKind::Identifier(id) if id == "Some" => {
                // Handle Some(value)
                self.advance(); // consume Some
                if matches!(self.peek(), TokenKind::Punctuation('(')) {
                    self.advance(); // consume (
                    let value = self.parse_expr()?;
                    if !self.expect_punctuation(')', "to close Some(...)") {
                        return None;
                    }
                    Some(Expr::Some(Box::new(value)))
                } else {
                    self.push_diagnostic("Expected '(' after 'Some'");
                    None
                }
            }
            TokenKind::Punctuation('(') => {
                // Handle parenthesized expressions for grouping
                self.advance(); // consume (
                let expr = self.parse_expr();
                if !self.expect_punctuation(')', "to close parenthesized expression") {
                    return None;
                }
                expr
            }
            TokenKind::InterpolatedString(_) => {
                // Handle interpolated strings - extract parts first to avoid borrow issues
                let parts = if let TokenKind::InterpolatedString(p) = self.peek() {
                    p.clone()
                } else {
                    Vec::new()
                };
                self.advance(); // consume the token
                self.parse_interpolated_string(parts)
            }
            _ => {
                // For other tokens, advance and match
                match self.advance() {
                    TokenKind::Identifier(name) => Some(Expr::Identifier(name.clone())),
                    TokenKind::Int(n) => Some(Expr::Int(*n)),
                    TokenKind::Float(n) => Some(Expr::Float(*n)),
                    TokenKind::String(s) => Some(Expr::String(s.clone())),
                    TokenKind::Bool(b) => Some(Expr::Bool(*b)),
                    _ => {
                        self.push_diagnostic("Expected expression");
                        None
                    }
                }
            }
        }
    }

    fn parse_array_literal(&mut self) -> Option<Expr> {
        self.with_expression_depth("array literal", |parser| parser.parse_array_literal_inner())
    }

    fn parse_array_literal_inner(&mut self) -> Option<Expr> {
        self.advance(); // consume [
        let mut elements = Vec::new();

        while !matches!(self.peek(), TokenKind::Punctuation(']'))
            && !matches!(self.peek(), TokenKind::Eof)
        {
            // Check for spread operator: ...expr
            if matches!(self.peek(), TokenKind::Operator(op) if op == "...") {
                self.advance(); // consume ...
                if let Some(expr) = self.parse_equality() {
                    elements.push(crate::ast::ArrayElement::Spread(expr));
                    if elements.len() > self.max_collection_literal_items {
                        self.push_diagnostic(format!(
                            "Array literal exceeds maximum element count of {}",
                            self.max_collection_literal_items
                        ));
                        return None;
                    }
                }
            } else if let Some(elem) = self.parse_equality() {
                elements.push(crate::ast::ArrayElement::Single(elem));
                if elements.len() > self.max_collection_literal_items {
                    self.push_diagnostic(format!(
                        "Array literal exceeds maximum element count of {}",
                        self.max_collection_literal_items
                    ));
                    return None;
                }
            }

            if matches!(self.peek(), TokenKind::Punctuation(',')) {
                self.advance();
            } else if !matches!(self.peek(), TokenKind::Punctuation(']')) {
                break; // Unexpected token
            }
        }

        if !self.expect_punctuation(']', "to close array literal") {
            return None;
        }

        Some(Expr::ArrayLiteral(elements))
    }

    fn parse_dict_literal(&mut self) -> Option<Expr> {
        self.with_expression_depth("dictionary literal", |parser| parser.parse_dict_literal_inner())
    }

    fn parse_dict_literal_inner(&mut self) -> Option<Expr> {
        self.advance(); // consume {
        let mut pairs = Vec::new();

        while !matches!(self.peek(), TokenKind::Punctuation('}'))
            && !matches!(self.peek(), TokenKind::Eof)
        {
            // Check for spread operator: ...expr
            if matches!(self.peek(), TokenKind::Operator(op) if op == "...") {
                self.advance(); // consume ...
                if let Some(expr) = self.parse_equality() {
                    pairs.push(crate::ast::DictElement::Spread(expr));
                    if pairs.len() > self.max_collection_literal_items {
                        self.push_diagnostic(format!(
                            "Dictionary literal exceeds maximum item count of {}",
                            self.max_collection_literal_items
                        ));
                        return None;
                    }
                }

                if matches!(self.peek(), TokenKind::Punctuation(',')) {
                    self.advance();
                } else if !matches!(self.peek(), TokenKind::Punctuation('}')) {
                    break;
                }
                continue;
            }
            // Parse key - use parse_equality to avoid recursion
            let key = self.parse_equality()?;

            // Expect colon
            if !matches!(self.peek(), TokenKind::Punctuation(':')) {
                self.push_diagnostic("Expected ':' in dictionary literal");
                break;
            }
            self.advance(); // consume :

            // Parse value - use parse_equality to avoid recursion
            let value = self.parse_equality()?;
            pairs.push(crate::ast::DictElement::Pair(key, value));
            if pairs.len() > self.max_collection_literal_items {
                self.push_diagnostic(format!(
                    "Dictionary literal exceeds maximum item count of {}",
                    self.max_collection_literal_items
                ));
                return None;
            }

            if matches!(self.peek(), TokenKind::Punctuation(',')) {
                self.advance();
            } else if !matches!(self.peek(), TokenKind::Punctuation('}')) {
                break;
            }
        }

        if !self.expect_punctuation('}', "to close dictionary literal") {
            return None;
        }

        Some(Expr::DictLiteral(pairs))
    }

    fn parse_interpolated_string(
        &mut self,
        parts: Vec<crate::lexer::InterpolatedPart>,
    ) -> Option<Expr> {
        use crate::ast::InterpolatedStringPart;
        use crate::lexer::InterpolatedPart as LexerPart;

        let mut ast_parts = Vec::new();
        let mut had_error = false;

        for part in parts {
            match part {
                LexerPart::Text(text) => {
                    ast_parts.push(InterpolatedStringPart::Text(text));
                }
                LexerPart::Expression(expr_str) => {
                    // Parse the expression string
                    match crate::lexer::tokenize(&expr_str) {
                        Ok(tokens) => {
                            let mut parser = Parser::new(tokens);
                            if let Some(expr) = parser.parse_expr() {
                                if matches!(parser.peek(), TokenKind::Eof) {
                                    ast_parts.push(InterpolatedStringPart::Expr(Box::new(expr)));
                                } else {
                                    had_error = true;
                                    self.push_diagnostic(
                                        "Invalid interpolated string expression".to_string(),
                                    );
                                }
                            } else {
                                had_error = true;
                                if parser.diagnostics.is_empty() {
                                    self.push_diagnostic(
                                        "Invalid interpolated string expression".to_string(),
                                    );
                                } else {
                                    for diagnostic in parser.diagnostics {
                                        self.push_diagnostic(diagnostic.message);
                                    }
                                }
                            }
                        }
                        Err(diagnostics) => {
                            had_error = true;
                            if diagnostics.is_empty() {
                                self.push_diagnostic(
                                    "Invalid interpolated string expression".to_string(),
                                );
                            } else {
                                for diagnostic in diagnostics {
                                    self.push_diagnostic(diagnostic.message);
                                }
                            }
                        }
                    }
                }
            }
        }

        if had_error {
            None
        } else {
            Some(Expr::InterpolatedString(ast_parts))
        }
    }

    /// Parse a type annotation (: type_name)
    /// Returns Some(TypeAnnotation) if a type annotation is present, None otherwise
    fn parse_type_annotation(&mut self) -> Option<crate::ast::TypeAnnotation> {
        use crate::ast::TypeAnnotation;

        // Check if there's a colon for type annotation
        // But NOT if it's := (assignment operator)
        if !matches!(self.peek(), TokenKind::Punctuation(':')) {
            return None;
        }

        // Peek ahead - if next token is '=', this is ':=' not a type annotation
        let saved_pos = self.pos;
        self.advance(); // tentatively consume :

        // Check if this is actually part of ':='
        if matches!(self.peek(), TokenKind::Operator(op) if op == "=") {
            // This was part of ':=', backtrack
            self.pos = saved_pos;
            return None;
        }

        // Parse the type keyword
        match self.peek() {
            TokenKind::Keyword(k) if k == "int" => {
                self.advance();
                Some(TypeAnnotation::Int)
            }
            TokenKind::Keyword(k) if k == "float" => {
                self.advance();
                Some(TypeAnnotation::Float)
            }
            TokenKind::Keyword(k) if k == "string" => {
                self.advance();
                Some(TypeAnnotation::String)
            }
            TokenKind::Keyword(k) if k == "bool" => {
                self.advance();
                Some(TypeAnnotation::Bool)
            }
            TokenKind::Keyword(k) if k == "Result" => self.parse_result_type_annotation(),
            TokenKind::Identifier(i) if i == "Result" => self.parse_result_type_annotation(),
            TokenKind::Keyword(k) if k == "Option" => self.parse_option_type_annotation(),
            TokenKind::Identifier(i) if i == "Option" => self.parse_option_type_annotation(),
            _ => {
                // Invalid type, backtrack
                self.pos = saved_pos;
                None
            }
        }
    }

    fn parse_result_type_annotation(&mut self) -> Option<crate::ast::TypeAnnotation> {
        use crate::ast::TypeAnnotation;

        self.advance(); // consume Result
                        // Expect Result<T, E> syntax
        if !matches!(self.peek(), TokenKind::Operator(op) if op == "<") {
            self.push_diagnostic("Expected '<' in Result<T, E> type annotation");
            return None;
        }
        self.advance(); // consume <

        let ok_type = self.parse_type_annotation_inner()?;

        if !matches!(self.peek(), TokenKind::Punctuation(',')) {
            self.push_diagnostic("Expected ',' in Result<T, E> type annotation");
            return None;
        }
        self.advance(); // consume ,

        let err_type = self.parse_type_annotation_inner()?;

        if !matches!(self.peek(), TokenKind::Operator(op) if op == ">") {
            self.push_diagnostic("Expected '>' in Result<T, E> type annotation");
            return None;
        }
        self.advance(); // consume >

        Some(TypeAnnotation::Result { ok_type: Box::new(ok_type), err_type: Box::new(err_type) })
    }

    fn parse_option_type_annotation(&mut self) -> Option<crate::ast::TypeAnnotation> {
        use crate::ast::TypeAnnotation;

        self.advance(); // consume Option
                        // Expect Option<T> syntax
        if !matches!(self.peek(), TokenKind::Operator(op) if op == "<") {
            self.push_diagnostic("Expected '<' in Option<T> type annotation");
            return None;
        }
        self.advance(); // consume <

        let inner_type = self.parse_type_annotation_inner()?;

        if !matches!(self.peek(), TokenKind::Operator(op) if op == ">") {
            self.push_diagnostic("Expected '>' in Option<T> type annotation");
            return None;
        }
        self.advance(); // consume >

        Some(TypeAnnotation::Option { inner_type: Box::new(inner_type) })
    }

    /// Parse a type annotation without consuming a leading colon (used inside generics)
    fn parse_type_annotation_inner(&mut self) -> Option<crate::ast::TypeAnnotation> {
        use crate::ast::TypeAnnotation;

        match self.peek() {
            TokenKind::Keyword(k) if k == "int" => {
                self.advance();
                Some(TypeAnnotation::Int)
            }
            TokenKind::Keyword(k) if k == "float" => {
                self.advance();
                Some(TypeAnnotation::Float)
            }
            TokenKind::Keyword(k) if k == "string" => {
                self.advance();
                Some(TypeAnnotation::String)
            }
            TokenKind::Keyword(k) if k == "bool" => {
                self.advance();
                Some(TypeAnnotation::Bool)
            }
            TokenKind::Keyword(k) if k == "Result" => self.parse_result_type_annotation(),
            TokenKind::Identifier(i) if i == "Result" => self.parse_result_type_annotation(),
            TokenKind::Keyword(k) if k == "Option" => self.parse_option_type_annotation(),
            TokenKind::Identifier(i) if i == "Option" => self.parse_option_type_annotation(),
            _ => None,
        }
    }

    // --- TEST RUNNER ---

    fn run_test_fixture(
        current_exe: &Path,
        fixture_path: &Path,
        use_interpreter: bool,
    ) -> Result<String, String> {
        let mut command = Command::new(current_exe);
        command.arg("run").arg(fixture_path);
        if use_interpreter {
            command.arg("--interpreter");
        }

        match command.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !output.status.success() && stdout.is_empty() && stderr.is_empty() {
                    let exit_code = output.status.code().unwrap_or(-1);
                    return Err(format!(
                        "Fixture {} exited with code {} and produced no output",
                        fixture_path.display(),
                        exit_code
                    ));
                }

                match (stdout.is_empty(), stderr.is_empty()) {
                    (false, false) => Ok(format!("{stdout}\n{stderr}")),
                    (false, true) => Ok(stdout),
                    (true, false) => Ok(stderr),
                    (true, true) => Ok(String::new()),
                }
            }
            Err(err) => Err(format!("Failed to execute test script: {err}")),
        }
    }

    fn normalize_fixture_output(output: &str) -> String {
        output
            .lines()
            .filter(|line| !line.starts_with("Compiler optimization:"))
            .map(|line| {
                if let Some(start) = line.find(" (took ~") {
                    if let Some(end_rel) = line[start..].find("ms)") {
                        let end = start + end_rel + 3;
                        return format!("{}{}", &line[..start], &line[end..]);
                    }
                }
                line.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    pub fn run_all_tests(
        test_dir: &Path,
        update_snapshots: bool,
        runtime_strategy: TestRuntimeStrategy,
        verbose: bool,
    ) -> i32 {
        let Ok(entries) = fs::read_dir(test_dir) else {
            eprintln!("[!] Failed to read test directory: {}", test_dir.display());
            return 5;
        };

        let mut passed = 0;
        let mut total = 0;
        let mut vm_primary_passed = 0;
        let mut interpreter_primary_passed = 0;
        let mut dual_fallback_passed = 0;
        let mut had_failures = false;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "kujo").unwrap_or(false) {
                if verbose {
                    println!("[i] Running fixture: {}", path.display());
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                if content.lines().take(3).any(|line| line.contains("Run with: kujo test-run")) {
                    if verbose {
                        println!("[i] Skipping test-run fixture: {}", path.display());
                    }
                    continue;
                }
                total += 1;
                let expected_path = path.with_extension("out");

                let tokens = match crate::lexer::tokenize(&content) {
                    Ok(tokens) => tokens,
                    Err(diagnostics) => {
                        println!("[✗] {}", path.display());
                        for diagnostic in diagnostics {
                            println!(
                                "Lexer error at {}:{}: {}",
                                diagnostic.line, diagnostic.column, diagnostic.message
                            );
                        }
                        had_failures = true;
                        continue;
                    }
                };
                let mut parser = crate::parser::Parser::new(tokens);
                let parse_output = parser.parse_with_diagnostics();
                if !parse_output.diagnostics.is_empty() {
                    println!("[✗] {}", path.display());
                    for diagnostic in parse_output.diagnostics {
                        println!(
                            "Parser error at {}:{}: {}",
                            diagnostic.line, diagnostic.column, diagnostic.message
                        );
                    }
                    had_failures = true;
                    continue;
                }
                let start = Instant::now();
                // Run each fixture in a child process so one runtime crash (for example stack
                // overflow in a specific script) does not abort the whole `kujo test` run.
                let current_exe = match std::env::current_exe() {
                    Ok(path) => path,
                    Err(err) => {
                        println!("[✗] {}", path.display());
                        println!("Failed to locate current executable: {err}");
                        had_failures = true;
                        continue;
                    }
                };

                let expected_exists = expected_path.exists();
                let expected = if expected_exists && !update_snapshots {
                    fs::read_to_string(&expected_path).unwrap_or_default().trim().to_string()
                } else {
                    String::new()
                };
                let expected = Self::normalize_fixture_output(&expected);

                if update_snapshots || !expected_exists {
                    let snapshot_output = match runtime_strategy {
                        TestRuntimeStrategy::Interpreter => {
                            match Self::run_test_fixture(&current_exe, &path, true) {
                                Ok(output) => output,
                                Err(err) => {
                                    println!("[✗] {}", path.display());
                                    println!("{err}");
                                    had_failures = true;
                                    continue;
                                }
                            }
                        }
                        TestRuntimeStrategy::Vm => {
                            match Self::run_test_fixture(&current_exe, &path, false) {
                                Ok(output) => output,
                                Err(err) => {
                                    println!("[✗] {}", path.display());
                                    println!("{err}");
                                    had_failures = true;
                                    continue;
                                }
                            }
                        }
                        TestRuntimeStrategy::Dual => {
                            // In update/new-snapshot mode, preserve legacy interpreter snapshot
                            // shape until a dedicated snapshot migration is completed.
                            match Self::run_test_fixture(&current_exe, &path, true) {
                                Ok(output) => output,
                                Err(err) => {
                                    println!("[✗] {}", path.display());
                                    println!("{err}");
                                    had_failures = true;
                                    continue;
                                }
                            }
                        }
                    };
                    let snapshot_output = Self::normalize_fixture_output(&snapshot_output);
                    fs::write(&expected_path, &snapshot_output).ok();
                    println!("[✓] {} ({:.2?})", path.display(), start.elapsed());
                    if verbose {
                        println!("    snapshot: {}", expected_path.display());
                    }
                    passed += 1;
                    continue;
                }

                let mut actual_for_report: Option<String> = None;
                let mut fallback_for_report: Option<String> = None;
                let mut used_interpreter_fallback = false;
                let matched = match runtime_strategy {
                    TestRuntimeStrategy::Interpreter => {
                        let actual = match Self::run_test_fixture(&current_exe, &path, true) {
                            Ok(output) => output,
                            Err(err) => {
                                println!("[✗] {}", path.display());
                                println!("{err}");
                                had_failures = true;
                                continue;
                            }
                        };
                        let actual = Self::normalize_fixture_output(&actual);
                        let is_match = actual == expected;
                        if is_match {
                            interpreter_primary_passed += 1;
                        } else {
                            actual_for_report = Some(actual);
                        }
                        is_match
                    }
                    TestRuntimeStrategy::Vm => {
                        let actual = match Self::run_test_fixture(&current_exe, &path, false) {
                            Ok(output) => output,
                            Err(err) => {
                                println!("[✗] {}", path.display());
                                println!("{err}");
                                had_failures = true;
                                continue;
                            }
                        };
                        let actual = Self::normalize_fixture_output(&actual);
                        let is_match = actual == expected;
                        if is_match {
                            vm_primary_passed += 1;
                        } else {
                            actual_for_report = Some(actual);
                        }
                        is_match
                    }
                    TestRuntimeStrategy::Dual => {
                        let vm_actual = match Self::run_test_fixture(&current_exe, &path, false) {
                            Ok(output) => output,
                            Err(err) => {
                                println!("[✗] {}", path.display());
                                println!("{err}");
                                had_failures = true;
                                continue;
                            }
                        };
                        let vm_actual = Self::normalize_fixture_output(&vm_actual);
                        if vm_actual == expected {
                            vm_primary_passed += 1;
                            true
                        } else {
                            let interpreter_actual =
                                match Self::run_test_fixture(&current_exe, &path, true) {
                                    Ok(output) => output,
                                    Err(err) => {
                                        println!("[✗] {}", path.display());
                                        println!("{err}");
                                        had_failures = true;
                                        continue;
                                    }
                                };
                            let interpreter_actual =
                                Self::normalize_fixture_output(&interpreter_actual);
                            if interpreter_actual == expected {
                                dual_fallback_passed += 1;
                                used_interpreter_fallback = true;
                                true
                            } else {
                                actual_for_report = Some(vm_actual);
                                fallback_for_report = Some(interpreter_actual);
                                false
                            }
                        }
                    }
                };

                if matched {
                    if matches!(runtime_strategy, TestRuntimeStrategy::Dual)
                        && used_interpreter_fallback
                    {
                        println!(
                            "[✓] {} ({:.2?}) [dual fallback: interpreter]",
                            path.display(),
                            start.elapsed()
                        );
                    } else {
                        println!("[✓] {} ({:.2?})", path.display(), start.elapsed());
                    }
                    if verbose {
                        println!("    snapshot: {}", expected_path.display());
                    }
                    passed += 1;
                } else {
                    println!("[✗] {}", path.display());
                    println!("Expected:\n{}", expected);
                    if let Some(actual) = actual_for_report {
                        println!("Got ({})\n{}\n", runtime_strategy.label(), actual);
                    }
                    if let Some(fallback_actual) = fallback_for_report {
                        println!("Got (interpreter fallback)\n{}\n", fallback_actual);
                    }
                    had_failures = true;
                }
            }
        }

        println!("\n[✓] Passed {}/{} tests", passed, total);
        if matches!(runtime_strategy, TestRuntimeStrategy::Dual) {
            println!(
                "[i] Runtime strategy: dual (vm_primary={}, interpreter_fallback={})",
                vm_primary_passed, dual_fallback_passed
            );
        } else if matches!(runtime_strategy, TestRuntimeStrategy::Vm) {
            println!("[i] Runtime strategy: vm (vm_primary={})", vm_primary_passed);
        } else {
            println!(
                "[i] Runtime strategy: interpreter (interpreter_primary={})",
                interpreter_primary_passed
            );
        }

        if had_failures {
            3
        } else {
            0
        }
    }
}
