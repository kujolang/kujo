use crate::lexer::{self, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLocation {
    pub line: usize,
    pub column: usize,
    pub is_definition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolDeclaration {
    name: String,
    line: usize,
    column: usize,
    token_index: usize,
    scope_path: Vec<usize>,
}

pub fn find_references(
    source: &str,
    line: usize,
    column: usize,
    include_definition: bool,
) -> Vec<ReferenceLocation> {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(_) => return Vec::new(),
    };
    let (scope_before_token, scope_after_token) = collect_scope_paths(&tokens);
    let declarations =
        collect_symbol_declarations(&tokens, &scope_before_token, &scope_after_token);

    let target_token_index = match identifier_token_index_at_cursor(&tokens, line, column) {
        Some(index) => index,
        None => return Vec::new(),
    };

    let target_definition = match resolve_declaration_for_token(
        &tokens,
        target_token_index,
        &declarations,
        &scope_before_token,
    ) {
        Some(definition) => definition,
        None => return Vec::new(),
    };
    let mut references = Vec::new();

    for (token_index, token) in tokens.iter().enumerate() {
        let identifier_name = match &token.kind {
            TokenKind::Identifier(name) => name,
            _ => continue,
        };

        if identifier_name != &target_definition.name {
            continue;
        }

        let resolved_declaration = match resolve_declaration_for_token(
            &tokens,
            token_index,
            &declarations,
            &scope_before_token,
        ) {
            Some(definition) => definition,
            None => continue,
        };

        if !is_same_declaration(&resolved_declaration, &target_definition) {
            continue;
        }

        let token_start_column = identifier_start_column(token, identifier_name);
        if token_start_column == 0 {
            continue;
        }

        let is_definition =
            token.line == target_definition.line && token_start_column == target_definition.column;

        if !include_definition && is_definition {
            continue;
        }

        references.push(ReferenceLocation {
            line: token.line,
            column: token_start_column,
            is_definition,
        });
    }

    references.sort_by_key(|reference| (reference.line, reference.column));
    references
}

fn identifier_token_index_at_cursor(tokens: &[Token], line: usize, column: usize) -> Option<usize> {
    for (index, token) in tokens.iter().enumerate() {
        if token.line != line {
            continue;
        }

        let name = match &token.kind {
            TokenKind::Identifier(name) => name,
            _ => continue,
        };

        let start = identifier_start_column(token, name);
        if start == 0 {
            continue;
        }

        let end = token.column.saturating_sub(1);
        if (start..=token.column).contains(&column) || (start..=end).contains(&column) {
            return Some(index);
        }
    }

    None
}

fn identifier_start_column(token: &Token, name: &str) -> usize {
    token.column.saturating_sub(name.chars().count())
}

fn collect_scope_paths(tokens: &[Token]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut scope_before_token = Vec::with_capacity(tokens.len());
    let mut scope_after_token = Vec::with_capacity(tokens.len());

    let mut scope_stack: Vec<usize> = vec![0];
    let mut next_scope_id: usize = 1;

    for token in tokens.iter() {
        scope_before_token.push(scope_stack.clone());

        match token.kind {
            TokenKind::Punctuation('{') => {
                scope_stack.push(next_scope_id);
                next_scope_id += 1;
            }
            TokenKind::Punctuation('}') => {
                if scope_stack.len() > 1 {
                    scope_stack.pop();
                }
            }
            _ => {}
        }

        scope_after_token.push(scope_stack.clone());
    }

    (scope_before_token, scope_after_token)
}

fn collect_symbol_declarations(
    tokens: &[Token],
    scope_before_token: &[Vec<usize>],
    scope_after_token: &[Vec<usize>],
) -> Vec<SymbolDeclaration> {
    let mut declarations = Vec::new();
    let mut token_index = 0;

    while token_index < tokens.len() {
        if is_keyword(tokens, token_index, "async") && is_keyword(tokens, token_index + 1, "func") {
            collect_function_declarations(
                tokens,
                token_index + 1,
                scope_before_token,
                scope_after_token,
                &mut declarations,
            );
            token_index += 1;
        } else if is_keyword(tokens, token_index, "func") {
            collect_function_declarations(
                tokens,
                token_index,
                scope_before_token,
                scope_after_token,
                &mut declarations,
            );
        } else if is_keyword(tokens, token_index, "let") {
            collect_let_declaration(tokens, token_index, scope_before_token, &mut declarations);
        } else if is_keyword(tokens, token_index, "const") {
            collect_single_identifier_declaration(
                tokens,
                token_index + 1,
                scope_before_token,
                &mut declarations,
            );
        } else if is_keyword(tokens, token_index, "for") {
            collect_single_identifier_declaration(
                tokens,
                token_index + 1,
                scope_before_token,
                &mut declarations,
            );
        } else if is_keyword(tokens, token_index, "except") {
            collect_single_identifier_declaration(
                tokens,
                token_index + 1,
                scope_before_token,
                &mut declarations,
            );
        }

        token_index += 1;
    }

    declarations
}

fn collect_function_declarations(
    tokens: &[Token],
    function_keyword_index: usize,
    scope_before_token: &[Vec<usize>],
    scope_after_token: &[Vec<usize>],
    declarations: &mut Vec<SymbolDeclaration>,
) {
    let function_name_index = function_keyword_index + 1;
    collect_single_identifier_declaration(
        tokens,
        function_name_index,
        scope_before_token,
        declarations,
    );

    let mut parameter_token_indexes = Vec::new();
    let mut paren_depth: usize = 0;
    let mut expects_parameter_name = false;
    let mut token_index = function_name_index + 1;
    let mut function_body_scope: Option<Vec<usize>> = None;

    while token_index < tokens.len() {
        match &tokens[token_index].kind {
            TokenKind::Punctuation('(') => {
                paren_depth += 1;
                if paren_depth == 1 {
                    expects_parameter_name = true;
                }
            }
            TokenKind::Punctuation(')') => {
                if paren_depth == 1 {
                    expects_parameter_name = false;
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            TokenKind::Punctuation(',') => {
                if paren_depth == 1 {
                    expects_parameter_name = true;
                }
            }
            TokenKind::Identifier(_) if paren_depth == 1 && expects_parameter_name => {
                parameter_token_indexes.push(token_index);
                expects_parameter_name = false;
            }
            TokenKind::Punctuation('{') => {
                function_body_scope = scope_after_token.get(token_index).cloned();
                break;
            }
            _ => {}
        }

        token_index += 1;
    }

    if let Some(parameter_scope) = function_body_scope {
        for parameter_index in parameter_token_indexes {
            if let Some(declaration) =
                declaration_from_token(tokens, parameter_index, parameter_scope.clone())
            {
                declarations.push(declaration);
            }
        }
    }
}

fn collect_let_declaration(
    tokens: &[Token],
    let_keyword_index: usize,
    scope_before_token: &[Vec<usize>],
    declarations: &mut Vec<SymbolDeclaration>,
) {
    let mut declaration_index = let_keyword_index + 1;
    if is_keyword(tokens, declaration_index, "mut") {
        declaration_index += 1;
    }

    collect_single_identifier_declaration(
        tokens,
        declaration_index,
        scope_before_token,
        declarations,
    );
}

fn collect_single_identifier_declaration(
    tokens: &[Token],
    token_index: usize,
    scope_before_token: &[Vec<usize>],
    declarations: &mut Vec<SymbolDeclaration>,
) {
    let scope_path = match scope_before_token.get(token_index) {
        Some(scope) => scope.clone(),
        None => return,
    };

    if let Some(declaration) = declaration_from_token(tokens, token_index, scope_path) {
        declarations.push(declaration);
    }
}

fn declaration_from_token(
    tokens: &[Token],
    token_index: usize,
    scope_path: Vec<usize>,
) -> Option<SymbolDeclaration> {
    match tokens.get(token_index) {
        Some(Token { kind: TokenKind::Identifier(name), line, column, .. }) => {
            Some(SymbolDeclaration {
                name: name.clone(),
                line: *line,
                column: column.saturating_sub(name.chars().count()),
                token_index,
                scope_path,
            })
        }
        _ => None,
    }
}

fn resolve_declaration_for_token(
    tokens: &[Token],
    token_index: usize,
    declarations: &[SymbolDeclaration],
    scope_before_token: &[Vec<usize>],
) -> Option<SymbolDeclaration> {
    let token = tokens.get(token_index)?;
    let symbol_name = match &token.kind {
        TokenKind::Identifier(name) => name,
        _ => return None,
    };

    let usage_scope = scope_before_token.get(token_index)?;

    let mut best_visible_previous: Option<SymbolDeclaration> = None;
    let mut best_visible_fallback: Option<SymbolDeclaration> = None;

    for declaration in declarations.iter().filter(|entry| entry.name == *symbol_name) {
        if !is_scope_visible(&declaration.scope_path, usage_scope) {
            continue;
        }

        match &best_visible_fallback {
            Some(current) => {
                if declaration.token_index < current.token_index {
                    best_visible_fallback = Some(declaration.clone());
                }
            }
            None => {
                best_visible_fallback = Some(declaration.clone());
            }
        }

        if declaration.token_index <= token_index {
            match &best_visible_previous {
                Some(current) => {
                    let current_rank = (current.scope_path.len(), current.token_index);
                    let candidate_rank = (declaration.scope_path.len(), declaration.token_index);
                    if candidate_rank > current_rank {
                        best_visible_previous = Some(declaration.clone());
                    }
                }
                None => {
                    best_visible_previous = Some(declaration.clone());
                }
            }
        }
    }

    best_visible_previous.or(best_visible_fallback)
}

fn is_scope_visible(declaration_scope: &[usize], usage_scope: &[usize]) -> bool {
    declaration_scope.len() <= usage_scope.len()
        && declaration_scope.iter().zip(usage_scope.iter()).all(|(left, right)| left == right)
}

fn is_same_declaration(left: &SymbolDeclaration, right: &SymbolDeclaration) -> bool {
    left.name == right.name
        && left.line == right.line
        && left.column == right.column
        && left.token_index == right.token_index
}

fn is_keyword(tokens: &[Token], token_index: usize, keyword: &str) -> bool {
    match tokens.get(token_index) {
        Some(Token { kind: TokenKind::Keyword(current_keyword), .. }) => current_keyword == keyword,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::find_references;

    #[test]
    fn finds_all_function_references_including_definition() {
        let source = [
            "func greet(name) {",
            "    return name",
            "}",
            "let first := greet(\"kujo\")",
            "let second := greet(\"lang\")",
        ]
        .join("\n");

        let references = find_references(&source, 4, 16, true);
        let positions: Vec<(usize, usize, bool)> = references
            .iter()
            .map(|location| (location.line, location.column, location.is_definition))
            .collect();

        assert_eq!(positions, vec![(1, 6, true), (4, 14, false), (5, 15, false)]);
    }

    #[test]
    fn excludes_definition_when_requested() {
        let source = [
            "func greet(name) {",
            "    return name",
            "}",
            "let first := greet(\"kujo\")",
            "let second := greet(\"lang\")",
        ]
        .join("\n");

        let references = find_references(&source, 4, 16, false);
        let positions: Vec<(usize, usize, bool)> = references
            .iter()
            .map(|location| (location.line, location.column, location.is_definition))
            .collect();

        assert_eq!(positions, vec![(4, 14, false), (5, 15, false)]);
    }

    #[test]
    fn keeps_shadowed_variables_scoped_to_selected_definition() {
        let source = [
            "let value := 1",
            "func show() {",
            "    let value := 2",
            "    print(value)",
            "}",
            "print(value)",
        ]
        .join("\n");

        let inner_references = find_references(&source, 4, 12, true);
        let inner_positions: Vec<(usize, usize, bool)> = inner_references
            .iter()
            .map(|location| (location.line, location.column, location.is_definition))
            .collect();
        assert_eq!(inner_positions, vec![(3, 9, true), (4, 11, false)]);

        let outer_references = find_references(&source, 6, 7, true);
        let outer_positions: Vec<(usize, usize, bool)> = outer_references
            .iter()
            .map(|location| (location.line, location.column, location.is_definition))
            .collect();
        assert_eq!(outer_positions, vec![(1, 5, true), (6, 7, false)]);
    }

    #[test]
    fn returns_empty_when_cursor_is_not_symbol_reference() {
        let source = "let value := 1\nprint(value)\n";
        assert!(find_references(source, 2, 1, true).is_empty());
    }
}
