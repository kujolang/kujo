use crate::lexer::{self, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Function,
    Variable,
    Parameter,
}

impl DefinitionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefinitionKind::Function => "function",
            DefinitionKind::Variable => "variable",
            DefinitionKind::Parameter => "parameter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub kind: DefinitionKind,
}

pub fn find_definition(source: &str, line: usize, column: usize) -> Option<DefinitionLocation> {
    if line == 0 || column == 0 {
        return None;
    }

    let tokens = lexer::tokenize(source).ok()?;
    let identifier = identifier_at_cursor(&tokens, line, column)?;
    let definitions = collect_definitions(&tokens);

    let mut best_before_cursor: Option<DefinitionLocation> = None;
    let mut best_any: Option<DefinitionLocation> = None;

    for definition in definitions.into_iter().filter(|entry| entry.name == identifier) {
        match &best_any {
            Some(current) => {
                if is_earlier(&definition, current) {
                    best_any = Some(definition.clone());
                }
            }
            None => {
                best_any = Some(definition.clone());
            }
        }

        if is_before_or_equal(&definition, line, column) {
            match &best_before_cursor {
                Some(current) => {
                    if is_later(&definition, current) {
                        best_before_cursor = Some(definition);
                    }
                }
                None => {
                    best_before_cursor = Some(definition);
                }
            }
        }
    }

    best_before_cursor.or(best_any)
}

fn is_before_or_equal(location: &DefinitionLocation, line: usize, column: usize) -> bool {
    location.line < line || (location.line == line && location.column <= column)
}

fn is_earlier(candidate: &DefinitionLocation, reference: &DefinitionLocation) -> bool {
    candidate.line < reference.line
        || (candidate.line == reference.line && candidate.column < reference.column)
}

fn is_later(candidate: &DefinitionLocation, reference: &DefinitionLocation) -> bool {
    candidate.line > reference.line
        || (candidate.line == reference.line && candidate.column > reference.column)
}

fn identifier_at_cursor(tokens: &[Token], line: usize, column: usize) -> Option<String> {
    for token in tokens.iter() {
        if token.line != line {
            continue;
        }

        let name = match &token.kind {
            TokenKind::Identifier(name) => name,
            _ => continue,
        };

        let token_end_exclusive = token.column;
        let token_start = token_end_exclusive.saturating_sub(name.chars().count());
        let token_end_inclusive = token_end_exclusive.saturating_sub(1);

        if token_start == 0 {
            continue;
        }

        if (token_start..=token_end_exclusive).contains(&column)
            || (token_start..=token_end_inclusive).contains(&column)
        {
            return Some(name.clone());
        }
    }

    None
}

fn collect_definitions(tokens: &[Token]) -> Vec<DefinitionLocation> {
    let mut definitions = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if is_keyword(tokens, index, "async") && is_keyword(tokens, index + 1, "func") {
            collect_function_definition(tokens, index + 1, &mut definitions);
            index += 1;
        } else if is_keyword(tokens, index, "func") {
            collect_function_definition(tokens, index, &mut definitions);
        } else if is_keyword(tokens, index, "let") {
            collect_let_definition(tokens, index, &mut definitions);
        } else if is_keyword(tokens, index, "const") {
            collect_const_definition(tokens, index, &mut definitions);
        } else if is_keyword(tokens, index, "for") {
            collect_for_definition(tokens, index, &mut definitions);
        } else if is_keyword(tokens, index, "except") {
            collect_except_definition(tokens, index, &mut definitions);
        }

        index += 1;
    }

    definitions
}

fn collect_function_definition(
    tokens: &[Token],
    func_keyword_index: usize,
    definitions: &mut Vec<DefinitionLocation>,
) {
    let name_token_index = func_keyword_index + 1;
    if let Some(location) =
        identifier_definition(tokens, name_token_index, DefinitionKind::Function)
    {
        definitions.push(location);
    }

    let mut paren_depth: usize = 0;
    let mut expects_param_name = false;
    let mut token_index = name_token_index + 1;

    while token_index < tokens.len() {
        match &tokens[token_index].kind {
            TokenKind::Punctuation('{') => break,
            TokenKind::Punctuation('(') => {
                paren_depth += 1;
                if paren_depth == 1 {
                    expects_param_name = true;
                }
            }
            TokenKind::Punctuation(')') => {
                if paren_depth == 1 {
                    break;
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            TokenKind::Punctuation(',') => {
                if paren_depth == 1 {
                    expects_param_name = true;
                }
            }
            TokenKind::Identifier(_name) if paren_depth == 1 && expects_param_name => {
                if let Some(param_location) =
                    identifier_definition(tokens, token_index, DefinitionKind::Parameter)
                {
                    definitions.push(param_location);
                }
                expects_param_name = false;
            }
            _ => {}
        }

        token_index += 1;
    }
}

fn collect_let_definition(
    tokens: &[Token],
    let_keyword_index: usize,
    definitions: &mut Vec<DefinitionLocation>,
) {
    let mut next_index = let_keyword_index + 1;
    if is_keyword(tokens, next_index, "mut") {
        next_index += 1;
    }

    if let Some(location) = identifier_definition(tokens, next_index, DefinitionKind::Variable) {
        definitions.push(location);
    }
}

fn collect_const_definition(
    tokens: &[Token],
    const_keyword_index: usize,
    definitions: &mut Vec<DefinitionLocation>,
) {
    if let Some(location) =
        identifier_definition(tokens, const_keyword_index + 1, DefinitionKind::Variable)
    {
        definitions.push(location);
    }
}

fn collect_for_definition(
    tokens: &[Token],
    for_keyword_index: usize,
    definitions: &mut Vec<DefinitionLocation>,
) {
    if let Some(location) =
        identifier_definition(tokens, for_keyword_index + 1, DefinitionKind::Variable)
    {
        definitions.push(location);
    }
}

fn collect_except_definition(
    tokens: &[Token],
    except_keyword_index: usize,
    definitions: &mut Vec<DefinitionLocation>,
) {
    if let Some(location) =
        identifier_definition(tokens, except_keyword_index + 1, DefinitionKind::Variable)
    {
        definitions.push(location);
    }
}

fn identifier_definition(
    tokens: &[Token],
    token_index: usize,
    kind: DefinitionKind,
) -> Option<DefinitionLocation> {
    match tokens.get(token_index) {
        Some(Token { kind: TokenKind::Identifier(name), line, column, .. }) => {
            Some(DefinitionLocation {
                name: name.clone(),
                line: *line,
                column: column.saturating_sub(name.chars().count()),
                kind,
            })
        }
        _ => None,
    }
}

fn is_keyword(tokens: &[Token], token_index: usize, keyword: &str) -> bool {
    match tokens.get(token_index) {
        Some(Token { kind: TokenKind::Keyword(current), .. }) => current == keyword,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{find_definition, DefinitionKind};

    #[test]
    fn finds_function_definition_for_call_site() {
        let source =
            ["func greet(name) {", "    return name", "}", "let result := greet(\"kujo\")"]
                .join("\n");

        let definition = find_definition(&source, 4, 19).expect("expected function definition");
        assert_eq!(definition.name, "greet");
        assert_eq!(definition.line, 1);
        assert_eq!(definition.column, 6);
        assert_eq!(definition.kind, DefinitionKind::Function);
    }

    #[test]
    fn prefers_nearest_previous_definition_for_shadowed_variables() {
        let source =
            ["let value := 1", "func demo() {", "    let value := 2", "    print(value)", "}"]
                .join("\n");

        let definition =
            find_definition(&source, 4, 14).expect("expected nearest variable definition");
        assert_eq!(definition.name, "value");
        assert_eq!(definition.line, 3);
        assert_eq!(definition.column, 9);
        assert_eq!(definition.kind, DefinitionKind::Variable);
    }

    #[test]
    fn resolves_parameter_definition_from_function_body_usage() {
        let source = ["func square(value) {", "    return value * value", "}"].join("\n");

        let definition =
            find_definition(&source, 2, 14).expect("expected parameter definition location");
        assert_eq!(definition.name, "value");
        assert_eq!(definition.line, 1);
        assert_eq!(definition.column, 13);
        assert_eq!(definition.kind, DefinitionKind::Parameter);
    }

    #[test]
    fn falls_back_to_future_definition_when_no_previous_match_exists() {
        let source = ["print(run_later())", "func run_later() { return 1 }"].join("\n");

        let definition = find_definition(&source, 1, 8).expect("expected fallback definition");
        assert_eq!(definition.name, "run_later");
        assert_eq!(definition.line, 2);
        assert_eq!(definition.column, 6);
        assert_eq!(definition.kind, DefinitionKind::Function);
    }

    #[test]
    fn returns_none_when_cursor_is_not_on_identifier() {
        let source = "let value := 1\nprint(value)\n";
        assert!(find_definition(source, 2, 1).is_none());
    }
}
