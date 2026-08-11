use crate::lexer::{self, Token, TokenKind};
use crate::lsp_references;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEdit {
    pub line: usize,
    pub column: usize,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameResult {
    pub edits: Vec<RenameEdit>,
    pub updated_source: String,
}

pub fn rename_symbol(
    source: &str,
    line: usize,
    column: usize,
    new_name: &str,
) -> Result<RenameResult, String> {
    validate_identifier(new_name)?;

    let old_name = identifier_at_cursor(source, line, column)
        .ok_or_else(|| "No identifier found at cursor location".to_string())?;

    if old_name == new_name {
        return Ok(RenameResult { edits: Vec::new(), updated_source: source.to_string() });
    }

    let references = lsp_references::find_references(source, line, column, true);
    if references.is_empty() {
        return Err("No rename targets found for symbol under cursor".to_string());
    }

    let edits: Vec<RenameEdit> = references
        .iter()
        .map(|reference| RenameEdit {
            line: reference.line,
            column: reference.column,
            old_name: old_name.clone(),
            new_name: new_name.to_string(),
        })
        .collect();

    let updated_source = apply_rename_edits(source, &edits)?;
    Ok(RenameResult { edits, updated_source })
}

fn identifier_at_cursor(source: &str, line: usize, column: usize) -> Option<String> {
    let tokens = lexer::tokenize(source).ok()?;

    for token in tokens.iter() {
        if token.line != line {
            continue;
        }

        let name = match &token.kind {
            TokenKind::Identifier(name) => name,
            _ => continue,
        };

        let start = identifier_start_column(token, name);
        if 0 == start {
            continue;
        }
        if start <= column && column < token.column {
            return Some(name.clone());
        }
    }

    None
}

fn identifier_start_column(token: &Token, name: &str) -> usize {
    token.column.saturating_sub(name.chars().count())
}

fn validate_identifier(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("New symbol name must not be empty".to_string());
    }

    let mut chars = name.chars();
    let first_char = chars.next().unwrap();
    if !(first_char.is_ascii_alphabetic() || first_char == '_') {
        return Err("New symbol name must start with a letter or underscore".to_string());
    }

    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        return Err(
            "New symbol name must contain only letters, numbers, or underscores".to_string()
        );
    }

    let lexed = lexer::tokenize(name)
        .map_err(|_| "New symbol name must be a valid identifier".to_string())?;
    let token_is_identifier = matches!(
        lexed.as_slice(),
        [Token { kind: TokenKind::Identifier(_), .. }, Token { kind: TokenKind::Eof, .. }]
    );
    if !token_is_identifier {
        return Err("New symbol name must not be a reserved keyword".to_string());
    }

    Ok(())
}

fn apply_rename_edits(source: &str, edits: &[RenameEdit]) -> Result<String, String> {
    let mut lines: Vec<Vec<char>> = source.lines().map(|line| line.chars().collect()).collect();
    let line_had_trailing_newline = source.ends_with('\n');

    let mut grouped: Vec<(usize, Vec<&RenameEdit>)> = Vec::new();
    for edit in edits.iter() {
        if let Some((_, existing)) = grouped.iter_mut().find(|(line, _)| *line == edit.line) {
            existing.push(edit);
        } else {
            grouped.push((edit.line, vec![edit]));
        }
    }

    for (line_number, mut line_edits) in grouped.into_iter() {
        if 0 == line_number || line_number > lines.len() {
            return Err(format!("Rename target line {} is out of range", line_number));
        }

        line_edits.sort_by(|left, right| right.column.cmp(&left.column));

        for edit in line_edits.into_iter() {
            let line_index = line_number - 1;
            let char_start = edit.column.saturating_sub(1);
            let old_len = edit.old_name.chars().count();
            let char_end = char_start + old_len;
            let line_chars = &mut lines[line_index];

            if char_end > line_chars.len() {
                return Err(format!(
                    "Rename target {}:{} is out of bounds",
                    edit.line, edit.column
                ));
            }

            let current_segment: String = line_chars[char_start..char_end].iter().collect();
            if current_segment != edit.old_name {
                return Err(format!(
                    "Rename target {}:{} expected '{}' but found '{}'",
                    edit.line, edit.column, edit.old_name, current_segment
                ));
            }

            line_chars.splice(char_start..char_end, edit.new_name.chars());
        }
    }

    let mut result = lines
        .into_iter()
        .map(|chars| chars.into_iter().collect::<String>())
        .collect::<Vec<String>>()
        .join("\n");

    if line_had_trailing_newline {
        result.push('\n');
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::rename_symbol;

    #[test]
    fn renames_function_definition_and_call_sites() {
        let source = [
            "func greet(name) {",
            "    return name",
            "}",
            "let first := greet(\"kujo\")",
            "let second := greet(\"lang\")",
            "",
        ]
        .join("\n");

        let result = rename_symbol(&source, 4, 16, "welcome").expect("expected rename to succeed");
        assert_eq!(result.edits.len(), 3);
        assert!(result.updated_source.contains("func welcome(name)"));
        assert!(result.updated_source.contains("first := welcome(\"kujo\")"));
        assert!(result.updated_source.contains("second := welcome(\"lang\")"));
    }

    #[test]
    fn rename_stays_within_selected_shadow_scope() {
        let source = [
            "let value := 1",
            "func show() {",
            "    let value := 2",
            "    print(value)",
            "}",
            "print(value)",
            "",
        ]
        .join("\n");

        let result = rename_symbol(&source, 4, 12, "inner_value")
            .expect("expected scoped rename to succeed");

        assert_eq!(result.edits.len(), 2);
        assert!(result.updated_source.contains("let inner_value := 2"));
        assert!(result.updated_source.contains("print(inner_value)"));
        assert!(result.updated_source.contains("let value := 1"));
        assert!(result.updated_source.contains("print(value)"));
    }

    #[test]
    fn rejects_invalid_identifier_name() {
        let source = "let value := 1\nprint(value)\n";
        let error = rename_symbol(source, 1, 5, "123name").expect_err("expected invalid rename");
        assert!(error.contains("start with a letter or underscore"));
    }

    #[test]
    fn rejects_reserved_keyword_name() {
        let source = "let value := 1\nprint(value)\n";
        let error = rename_symbol(source, 1, 5, "if").expect_err("expected keyword rename error");
        assert!(error.contains("must not be a reserved keyword"));
    }

    #[test]
    fn returns_error_when_cursor_not_on_identifier() {
        let source = "let value := 1\nprint(value)\n";
        let error =
            rename_symbol(source, 1, 10, "renamed").expect_err("expected missing symbol error");
        assert!(error.contains("No identifier found"));
        let error =
            rename_symbol(source, 1, 11, "renamed").expect_err("expected missing symbol error");
        assert!(error.contains("No identifier found"));
    }
}
