use regex::Regex;

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

        if trimmed.starts_with('}') {
            indent_level = indent_level.saturating_sub(1);
        }

        let normalized = normalize_spacing(trimmed);
        let wrapped = wrap_if_needed(&normalized, indent_level, options);

        for (index, wrapped_line) in wrapped.into_iter().enumerate() {
            let continuation_indent = if index > 0 { 1 } else { 0 };
            let indent = " ".repeat(options.indent_width * (indent_level + continuation_indent));
            formatted_lines.push(format!("{}{}", indent, wrapped_line.trim()));
        }

        let opens = normalized.chars().filter(|ch| *ch == '{').count();
        let closes = normalized.chars().filter(|ch| *ch == '}').count();
        if opens > closes {
            indent_level += opens - closes;
        } else if closes > opens {
            indent_level = indent_level.saturating_sub(closes - opens);
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

    let mut result = line.to_string();

    let comma_regex = Regex::new(r",\s*").expect("comma regex must compile");
    result = comma_regex.replace_all(&result, ", ").to_string();

    let assignment_regex = Regex::new(r"\s*:=\s*").expect("assignment regex must compile");
    result = assignment_regex.replace_all(&result, " := ").to_string();

    let operators = ["==", "!=", ">=", "<=", "->", "+", "-", "*", "/", ">", "<"];
    for operator in operators.iter() {
        let escaped = regex::escape(operator);
        let pattern = format!(r"\s*{}\s*", escaped);
        let regex = Regex::new(&pattern).expect("operator regex must compile");
        result = regex.replace_all(&result, format!(" {} ", operator).as_str()).to_string();
    }

    result = result.replace("( ", "(").replace(" )", ")");
    result = result.replace("[ ", "[").replace(" ]", "]");
    result = result.replace("{ ", "{").replace(" }", "}");

    let whitespace_regex = Regex::new(r"\s+").expect("whitespace regex must compile");
    whitespace_regex.replace_all(result.trim(), " ").to_string()
}

fn wrap_if_needed(line: &str, indent_level: usize, options: &FormatterOptions) -> Vec<String> {
    let estimated_width = (indent_level * options.indent_width) + line.chars().count();
    if estimated_width <= options.line_length || !line.contains(", ") {
        return vec![line.to_string()];
    }

    let parts: Vec<&str> = line.split(", ").collect();
    if parts.len() <= 1 {
        return vec![line.to_string()];
    }

    let mut wrapped = Vec::new();
    let mut current = String::new();

    for (index, part) in parts.iter().enumerate() {
        let candidate =
            if current.is_empty() { part.to_string() } else { format!("{}, {}", current, part) };

        let candidate_width = (indent_level * options.indent_width) + candidate.chars().count();
        if candidate_width > options.line_length && !current.is_empty() {
            wrapped.push(current);
            current = part.to_string();
        } else {
            current = candidate;
        }

        if index == parts.len() - 1 && !current.is_empty() {
            wrapped.push(current.clone());
        }
    }

    if wrapped.is_empty() {
        vec![line.to_string()]
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::{format_source, FormatterOptions};

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
    fn formatter_wraps_long_comma_separated_lines() {
        let source = "print(a, b, c, d, e, f, g, h)\n";
        let formatted = format_source(
            source,
            &FormatterOptions { indent_width: 2, line_length: 20, sort_imports: false },
        );

        assert!(formatted.lines().count() > 1);
        assert!(formatted.contains("a, b, c"));
    }
}
