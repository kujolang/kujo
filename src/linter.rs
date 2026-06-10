use crate::lexer::{self, TokenKind};
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintSeverity {
    Warning,
    Error,
}

impl LintSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            LintSeverity::Warning => "warning",
            LintSeverity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFix {
    pub replacement_line: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintIssue {
    pub rule_id: String,
    pub line: usize,
    pub column: usize,
    pub severity: LintSeverity,
    pub message: String,
    pub fix: Option<LintFix>,
}

pub fn lint_source(source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    issues.extend(check_unused_variables(source));
    issues.extend(check_unreachable_code(source));
    issues.extend(check_obvious_type_mismatches(source));
    issues.extend(check_missing_error_handling_patterns(source));
    issues.sort_by_key(|issue| (issue.line, issue.column, issue.rule_id.clone()));
    issues
}

pub fn apply_safe_fixes(source: &str, issues: &[LintIssue]) -> String {
    let mut lines: Vec<String> = source.lines().map(|line| line.to_string()).collect();
    let trailing_newline = source.ends_with('\n');

    let mut replacement_by_line: HashMap<usize, String> = HashMap::new();
    for issue in issues.iter() {
        if let Some(fix) = &issue.fix {
            replacement_by_line.entry(issue.line).or_insert_with(|| fix.replacement_line.clone());
        }
    }

    for (line_number, replacement) in replacement_by_line.into_iter() {
        if 0 == line_number || line_number > lines.len() {
            continue;
        }
        lines[line_number - 1] = replacement;
    }

    let mut output = lines.join("\n");
    if trailing_newline {
        output.push('\n');
    }
    output
}

fn check_unused_variables(source: &str) -> Vec<LintIssue> {
    let tokens = match lexer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(diagnostics) => {
            return diagnostics
                .into_iter()
                .map(|diagnostic| LintIssue {
                    rule_id: "lexer-error".to_string(),
                    line: diagnostic.line,
                    column: diagnostic.column,
                    severity: LintSeverity::Error,
                    message: diagnostic.message,
                    fix: None,
                })
                .collect()
        }
    };
    let source_lines: Vec<&str> = source.lines().collect();

    let mut usage_counts: HashMap<String, usize> = HashMap::new();
    let mut declarations: Vec<(String, usize, usize)> = Vec::new();

    for token in tokens.iter() {
        if let TokenKind::Identifier(name) = &token.kind {
            *usage_counts.entry(name.clone()).or_insert(0) += 1;
        }
    }

    let mut index = 0;
    while index < tokens.len() {
        if matches!(&tokens[index].kind, TokenKind::Keyword(k) if k == "let") {
            let mut decl_index = index + 1;
            if matches!(tokens.get(decl_index).map(|t| &t.kind), Some(TokenKind::Keyword(k)) if k == "mut")
            {
                decl_index += 1;
            }
            if let Some(token) = tokens.get(decl_index) {
                if let TokenKind::Identifier(name) = &token.kind {
                    declarations.push((
                        name.clone(),
                        token.line,
                        token.column.saturating_sub(name.chars().count()),
                    ));
                }
            }
        } else if matches!(&tokens[index].kind, TokenKind::Keyword(k) if k == "const") {
            if let Some(token) = tokens.get(index + 1) {
                if let TokenKind::Identifier(name) = &token.kind {
                    declarations.push((
                        name.clone(),
                        token.line,
                        token.column.saturating_sub(name.chars().count()),
                    ));
                }
            }
        }
        index += 1;
    }

    let mut issues = Vec::new();
    for (name, line, column) in declarations.into_iter() {
        if name.starts_with('_') {
            continue;
        }

        if usage_counts.get(&name).copied().unwrap_or(0) <= 1 {
            let replacement_line = source_lines
                .get(line.saturating_sub(1))
                .map(|original| {
                    let needle = format!("let {}", name);
                    if original.contains(&needle) {
                        original.replacen(&needle, &format!("let _{}", name), 1)
                    } else {
                        original.to_string()
                    }
                })
                .unwrap_or_default();

            issues.push(LintIssue {
                rule_id: "unused-variable".to_string(),
                line,
                column,
                severity: LintSeverity::Warning,
                message: format!("Variable '{}' is declared but never used", name),
                fix: Some(LintFix {
                    replacement_line,
                    description: "Prefix unused variable with '_' to mark as intentional"
                        .to_string(),
                }),
            });
        }
    }

    issues
}

fn check_unreachable_code(source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();
    let mut in_unreachable_region = false;
    let mut brace_balance: i64 = 0;

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        let opening = trimmed.chars().filter(|ch| *ch == '{').count() as i64;
        let closing = trimmed.chars().filter(|ch| *ch == '}').count() as i64;

        if in_unreachable_region && !trimmed.starts_with('}') {
            issues.push(LintIssue {
                rule_id: "unreachable-code".to_string(),
                line: line_number,
                column: 1,
                severity: LintSeverity::Warning,
                message: "Statement appears unreachable after control-flow terminator".to_string(),
                fix: None,
            });
        }

        if trimmed.starts_with("return")
            || trimmed.starts_with("break")
            || trimmed.starts_with("continue")
        {
            in_unreachable_region = true;
            brace_balance = opening - closing;
            continue;
        }

        if in_unreachable_region {
            brace_balance += opening - closing;
            if brace_balance <= 0 && trimmed.contains('}') {
                in_unreachable_region = false;
                brace_balance = 0;
            }
        }
    }

    issues
}

fn check_obvious_type_mismatches(source: &str) -> Vec<LintIssue> {
    let int_string = Regex::new("let\\s+[A-Za-z_][A-Za-z0-9_]*\\s*:\\s*int\\s*:=\\s*\\\"")
        .expect("int-string mismatch regex must compile");
    let float_string = Regex::new("let\\s+[A-Za-z_][A-Za-z0-9_]*\\s*:\\s*float\\s*:=\\s*\\\"")
        .expect("float-string mismatch regex must compile");
    let bool_numeric = Regex::new("let\\s+[A-Za-z_][A-Za-z0-9_]*\\s*:\\s*bool\\s*:=\\s*[0-9]")
        .expect("bool-numeric mismatch regex must compile");

    let mut issues = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        if int_string.is_match(line) {
            issues.push(LintIssue {
                rule_id: "obvious-type-mismatch".to_string(),
                line: line_number,
                column: 1,
                severity: LintSeverity::Error,
                message: "Obvious type mismatch: int annotation assigned a string literal"
                    .to_string(),
                fix: None,
            });
        }
        if float_string.is_match(line) {
            issues.push(LintIssue {
                rule_id: "obvious-type-mismatch".to_string(),
                line: line_number,
                column: 1,
                severity: LintSeverity::Error,
                message: "Obvious type mismatch: float annotation assigned a string literal"
                    .to_string(),
                fix: None,
            });
        }
        if bool_numeric.is_match(line) {
            issues.push(LintIssue {
                rule_id: "obvious-type-mismatch".to_string(),
                line: line_number,
                column: 1,
                severity: LintSeverity::Error,
                message: "Obvious type mismatch: bool annotation assigned numeric literal"
                    .to_string(),
                fix: None,
            });
        }
    }

    issues
}

fn check_missing_error_handling_patterns(source: &str) -> Vec<LintIssue> {
    let mut issues = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        let line_number = index + 1;

        let has_error_prone_call = trimmed.contains("read_file(")
            || trimmed.contains("http_get(")
            || trimmed.contains("parse_json(");
        if !has_error_prone_call {
            continue;
        }

        let handled_inline =
            trimmed.starts_with("try") || trimmed.contains("?") || trimmed.contains("except");
        if handled_inline {
            continue;
        }

        let previous_line = if line_number > 1 {
            source.lines().nth(line_number - 2).unwrap_or("").trim()
        } else {
            ""
        };
        let handled_by_previous = previous_line.starts_with("try");
        if handled_by_previous {
            continue;
        }

        issues.push(LintIssue {
            rule_id: "missing-error-handling-pattern".to_string(),
            line: line_number,
            column: 1,
            severity: LintSeverity::Warning,
            message: "Potentially fallible call without explicit error-handling pattern"
                .to_string(),
            fix: None,
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::{apply_safe_fixes, lint_source};

    #[test]
    fn lint_reports_unused_variable_and_offers_fix() {
        let source = "let value := 1\nprint(1)\n";
        let issues = lint_source(source);

        assert!(issues.iter().any(|issue| issue.rule_id == "unused-variable"));
        let fixed = apply_safe_fixes(source, &issues);
        assert!(fixed.contains("let _value := 1"));
    }

    #[test]
    fn lint_reports_unreachable_code() {
        let source = ["func test() {", "    return 1", "    print(2)", "}"].join("\n");
        let issues = lint_source(&source);
        assert!(issues.iter().any(|issue| issue.rule_id == "unreachable-code"));
    }

    #[test]
    fn lint_reports_obvious_type_mismatch() {
        let source = "let count: int := \"oops\"\n";
        let issues = lint_source(source);
        assert!(issues.iter().any(|issue| issue.rule_id == "obvious-type-mismatch"));
    }

    #[test]
    fn lint_reports_missing_error_handling_pattern() {
        let source = "let content := read_file(\"missing.txt\")\n";
        let issues = lint_source(source);
        assert!(issues.iter().any(|issue| issue.rule_id == "missing-error-handling-pattern"));
    }
}
