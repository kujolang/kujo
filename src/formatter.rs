use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct FormatterOptions {
    pub indent_width: usize,
    pub line_length: usize,
    pub sort_imports: bool,
}

impl Default for FormatterOptions {
    fn default() -> Self {
        Self { indent_width: 4, line_length: 100, sort_imports: true }
    }
}

pub fn format_source(source: &str, options: &FormatterOptions) -> String {
    let trailing_newline = source.ends_with('\n');
    let mut lines: Vec<String> = source.lines().map(|line| line.trim_end().to_string()).collect();

    if options.sort_imports {
        sort_leading_import_block(&mut lines);
    }

    let mut formatted_lines: Vec<String> = Vec::new();
    let mut indent_level: usize = 0;

    for line in lines.into_iter() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            formatted_lines.push(String::new());
            continue;
        }

        let normalized = normalize_spacing(trimmed);
        let (structural_source, _) = protect_non_code(&normalized);
        let leading_closes = structural_source.chars().take_while(|ch| *ch == '}').count();
        indent_level = indent_level.saturating_sub(leading_closes);
        let wrapped = wrap_if_needed(&normalized, indent_level, options);

        for (index, wrapped_line) in wrapped.into_iter().enumerate() {
            let continuation_indent = if index > 0 { 1 } else { 0 };
            let indent = " ".repeat(options.indent_width * (indent_level + continuation_indent));
            formatted_lines.push(format!("{}{}", indent, wrapped_line.trim()));
        }

        let opens = structural_source.chars().filter(|ch| *ch == '{').count();
        let remaining_closes = structural_source
            .chars()
            .filter(|ch| *ch == '}')
            .count()
            .saturating_sub(leading_closes);
        if opens > remaining_closes {
            indent_level += opens - remaining_closes;
        } else if remaining_closes > opens {
            indent_level = indent_level.saturating_sub(remaining_closes - opens);
        }
    }

    let mut output = formatted_lines.join("\n");
    if trailing_newline {
        output.push('\n');
    }
    output
}

fn sort_leading_import_block(lines: &mut [String]) {
    let mut start_index: Option<usize> = None;
    let mut end_index: Option<usize> = None;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if start_index.is_none() {
                continue;
            }
            break;
        }

        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            if start_index.is_none() {
                start_index = Some(index);
            }
            end_index = Some(index + 1);
        } else {
            break;
        }
    }

    if let (Some(start), Some(end)) = (start_index, end_index) {
        let mut imports: Vec<String> = lines[start..end].to_vec();
        imports.sort();
        for (offset, import_line) in imports.into_iter().enumerate() {
            lines[start + offset] = import_line;
        }
    }
}

fn normalize_spacing(line: &str) -> String {
    if line.starts_with('#') || line.starts_with("//") {
        return line.to_string();
    }

    let (mut result, protected_fragments) = protect_non_code(line);

    static COMMA_REGEX: OnceLock<Regex> = OnceLock::new();
    static SPACED_OPERATOR_REGEX: OnceLock<Regex> = OnceLock::new();
    static COMPACT_OPERATOR_REGEX: OnceLock<Regex> = OnceLock::new();
    static SINGLE_OPERATOR_REGEX: OnceLock<Regex> = OnceLock::new();
    static WHITESPACE_REGEX: OnceLock<Regex> = OnceLock::new();

    result = COMMA_REGEX
        .get_or_init(|| Regex::new(r",\s*").expect("comma regex must compile"))
        .replace_all(&result, ", ")
        .to_string();

    // Protect multi-character operators before normalizing single-character
    // operators. The previous implementation rewrote `->` as `- >` and
    // changed operators inside string literals and inline comments.
    let spaced_operators = [":=", "==", "!=", ">=", "<=", "->", "+=", "-=", "*=", "/=", "%=", "=>"];
    let compact_operators = ["...", "??", "?.", "::", "&&", "||", "|>"];
    let single_operators = ["+", "-", "*", "/", ">", "<", "%", "=", "!", "?", "&", "|"];
    let mut operator_fragments: Vec<(String, String)> = Vec::new();
    protect_operator_group(
        &mut result,
        &spaced_operators,
        true,
        SPACED_OPERATOR_REGEX.get_or_init(|| {
            Regex::new(r"\s*(?::=|==|!=|>=|<=|->|\+=|-=|\*=|/=|%=|=>)\s*")
                .expect("spaced operator regex must compile")
        }),
        &mut operator_fragments,
    );
    protect_operator_group(
        &mut result,
        &compact_operators,
        false,
        COMPACT_OPERATOR_REGEX.get_or_init(|| {
            Regex::new(r"\s*(?:\.\.\.|\?\?|\?\.|::|&&|\|\||\|>)\s*")
                .expect("compact operator regex must compile")
        }),
        &mut operator_fragments,
    );
    protect_operator_group(
        &mut result,
        &single_operators,
        true,
        SINGLE_OPERATOR_REGEX.get_or_init(|| {
            Regex::new(r"\s*(?:\+|-|\*|/|>|<|%|=|!|\?|&|\|)\s*")
                .expect("single operator regex must compile")
        }),
        &mut operator_fragments,
    );

    result = result.replace("( ", "(").replace(" )", ")");
    result = result.replace("[ ", "[").replace(" ]", "]");
    result = result.replace("{ ", "{").replace(" }", "}");

    result = WHITESPACE_REGEX
        .get_or_init(|| Regex::new(r"\s+").expect("whitespace regex must compile"))
        .replace_all(result.trim(), " ")
        .to_string();

    for (placeholder, replacement) in operator_fragments.into_iter() {
        result = result.replace(&placeholder, &replacement);
    }
    restore_non_code(&result, &protected_fragments)
}

fn protect_non_code(line: &str) -> (String, Vec<(String, String)>) {
    let chars: Vec<char> = line.chars().collect();
    let mut output = String::new();
    let mut fragments: Vec<(String, String)> = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        let is_comment = chars[index] == '#'
            || (chars[index] == '/' && chars.get(index + 1).copied() == Some('/'));
        if chars[index] == '"' || is_comment {
            let start = index;
            if is_comment {
                index = chars.len();
            } else {
                index += 1;
                while index < chars.len() {
                    if chars[index] == '\\' {
                        index = (index + 2).min(chars.len());
                        continue;
                    }
                    index += 1;
                    if chars.get(index - 1).copied() == Some('"') {
                        break;
                    }
                }
            }
            let fragment: String = chars[start..index].iter().collect();
            let placeholder = format!("\u{e000}{}\u{e001}", fragments.len());
            output.push_str(&placeholder);
            fragments.push((placeholder, fragment));
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }

    (output, fragments)
}

fn restore_non_code(line: &str, fragments: &[(String, String)]) -> String {
    let mut output = line.to_string();
    for (placeholder, fragment) in fragments {
        output = output.replace(placeholder, fragment);
    }
    output
}

fn protect_operator_group(
    line: &mut String,
    operators: &[&str],
    spaced: bool,
    operator_regex: &Regex,
    fragments: &mut Vec<(String, String)>,
) {
    *line = operator_regex
        .replace_all(line, |captures: &regex::Captures<'_>| {
            let matched = captures.get(0).map_or("", |capture| capture.as_str());
            let operator = operators
                .iter()
                .copied()
                .find(|operator| matched.contains(operator))
                .expect("operator regex must contain an operator");
            let placeholder = format!("\u{e100}{}\u{e101}", fragments.len());
            let replacement = if spaced { format!(" {} ", operator) } else { operator.to_string() };
            fragments.push((placeholder.clone(), replacement));
            placeholder
        })
        .to_string();
}

fn wrap_if_needed(line: &str, _indent_level: usize, _options: &FormatterOptions) -> Vec<String> {
    // Wrapping requires syntax awareness: line-oriented splitting can move a
    // comma out of a multiline call, array, or dictionary. Keep the formatter
    // lossless until an AST-aware wrapping pass is available.
    let _ = _options.line_length;
    vec![line.to_string()]
}

#[cfg(test)]
mod tests {
    use super::{format_source, FormatterOptions};
    use crate::lexer::{self, TokenKind};

    #[test]
    fn formatter_normalizes_spacing_and_indentation() {
        let source = [
            "func greet(name){",
            "let result:=name+\"!\"",
            "if(result==name){",
            "print(result)",
            "}",
            "}",
            "",
        ]
        .join("\n");

        let formatted = format_source(
            &source,
            &FormatterOptions { indent_width: 2, line_length: 120, sort_imports: true },
        );

        assert!(formatted.contains("func greet(name){"));
        assert!(formatted.contains("let result := name + \"!\""));
        assert!(formatted.contains("if(result == name){"));
        assert!(formatted.contains("  print(result)"));
    }

    #[test]
    fn formatter_sorts_leading_import_block() {
        let source =
            ["import zeta", "from beta import b", "import alpha", "", "print(1)", ""].join("\n");
        let formatted = format_source(&source, &FormatterOptions::default());
        let lines: Vec<&str> = formatted.lines().collect();

        assert_eq!(lines[0], "from beta import b");
        assert_eq!(lines[1], "import alpha");
        assert_eq!(lines[2], "import zeta");
    }

    #[test]
    fn formatter_keeps_long_nested_comma_expressions_intact() {
        let source = "print(a, b, c, d, e, f, g, h)\n";
        let formatted = format_source(
            source,
            &FormatterOptions { indent_width: 2, line_length: 20, sort_imports: false },
        );

        assert_eq!(formatted.lines().count(), 1);
        assert!(formatted.contains("a, b, c"));
    }

    #[test]
    fn formatter_preserves_strings_comments_and_operator_tokens() {
        let source = r#"func run(value){
let command:=["sh","-lc","printf 'a/b --flag >= x\n'"] # path / flag >= text
if(value>=2){
print(command)
}
}
"#;
        let formatted = format_source(source, &FormatterOptions::default());
        let original_tokens = lexer::tokenize(source).expect("source should lex");
        let formatted_tokens = lexer::tokenize(&formatted).expect("formatted source should lex");
        let original_kinds: Vec<TokenKind> =
            original_tokens.into_iter().map(|token| token.kind).collect();
        let formatted_kinds: Vec<TokenKind> =
            formatted_tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(original_kinds, formatted_kinds);
        assert!(formatted.contains("a/b --flag >= x\\n"));
        assert!(formatted.contains("# path / flag >= text"));
        assert!(formatted.contains("value >= 2"));
    }

    #[test]
    fn formatter_does_not_split_nested_comma_expressions() {
        let source = "let value := contains(lower, \"/.aws\") == 1\n";
        let formatted = format_source(
            source,
            &FormatterOptions { indent_width: 4, line_length: 20, sort_imports: true },
        );
        let original_tokens = lexer::tokenize(source).expect("source should lex");
        let formatted_tokens = lexer::tokenize(&formatted).expect("formatted source should lex");
        let original_kinds: Vec<TokenKind> =
            original_tokens.into_iter().map(|token| token.kind).collect();
        let formatted_kinds: Vec<TokenKind> =
            formatted_tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(original_kinds, formatted_kinds);
        assert_eq!(formatted.lines().count(), 1);
    }

    #[test]
    fn formatter_keeps_outer_indent_after_closing_nested_block() {
        let source = ["func run(){", "if(true){", "print(1)", "}", "print(2)", "}", ""].join("\n");

        let formatted = format_source(
            &source,
            &FormatterOptions { indent_width: 2, line_length: 100, sort_imports: false },
        );

        assert!(formatted.contains("\n  print(2)\n}"));
    }

    #[test]
    fn formatter_ignores_braces_inside_strings_and_comments_for_indentation() {
        let source =
            ["func run(){", "print(\"{\") # }", "print(1)", "}", "print(2)", ""].join("\n");

        let formatted = format_source(
            &source,
            &FormatterOptions { indent_width: 2, line_length: 100, sort_imports: false },
        );

        assert!(formatted.contains("\n  print(1)\n}"));
        assert!(formatted.contains("\nprint(2)\n"));
    }
}
