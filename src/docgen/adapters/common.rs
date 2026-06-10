use crate::docgen::model::{
    DocComment, DocCommentBlock, DocExample, DocGapKind, DocSymbol, DocVisibility,
};

pub fn doc_summary(lines: &[String]) -> Option<String> {
    lines.iter().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn extract_examples(lines: &[String]) -> Vec<DocExample> {
    let mut examples = Vec::new();
    let mut current = Vec::new();
    let mut language: Option<String> = None;
    let mut in_block = false;

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("```") {
            if in_block {
                if !current.is_empty() {
                    examples
                        .push(DocExample { language: language.clone(), code: current.join("\n") });
                    current.clear();
                }
                in_block = false;
                language = None;
            } else {
                in_block = true;
                let lang = rest.trim();
                if !lang.is_empty() {
                    language = Some(lang.to_string());
                }
            }
            continue;
        }
        if in_block {
            current.push(line.clone());
        }
    }

    if in_block && !current.is_empty() {
        examples.push(DocExample { language, code: current.join("\n") });
    }

    examples
}

pub fn placeholder_comment() -> DocComment {
    DocComment {
        lines: vec![
            "Documentation needed.".to_string(),
            "This symbol was discovered from the source code, but no human-authored documentation was found.".to_string(),
        ],
        summary: Some("Documentation needed.".to_string()),
        placeholder: true,
    }
}

pub fn attach_docs_by_proximity(
    mut symbols: Vec<DocSymbol>,
    docs: Vec<DocCommentBlock>,
) -> Vec<DocSymbol> {
    let mut sorted_docs = docs;
    sorted_docs.sort_by_key(|block| block.end_line);

    for symbol in &mut symbols {
        let doc_match = sorted_docs
            .iter()
            .filter(|block| block.end_line < symbol.line)
            .filter(|block| block.target_line_hint.is_none_or(|hint| hint == symbol.line))
            .max_by_key(|block| block.end_line);

        if let Some(block) = doc_match {
            symbol.docs = DocComment {
                summary: doc_summary(&block.lines),
                lines: block.lines.clone(),
                placeholder: false,
            };
            symbol.examples = extract_examples(&block.lines);
        } else {
            symbol.docs = placeholder_comment();
        }

        let mut gaps = Vec::new();
        if symbol.docs.placeholder {
            gaps.push(DocGapKind::MissingDocs);
            gaps.push(DocGapKind::MissingSummary);
            gaps.push(DocGapKind::MissingExamples);
        } else {
            if symbol.docs.summary.is_none() {
                gaps.push(DocGapKind::MissingSummary);
            }
            if symbol.examples.is_empty() {
                gaps.push(DocGapKind::MissingExamples);
            }
        }
        symbol.gaps = gaps;
    }

    symbols
}

pub fn next_nonempty_line(source: &str, from_line: usize) -> Option<usize> {
    for (idx, line) in source.lines().enumerate().skip(from_line) {
        if !line.trim().is_empty() {
            return Some(idx + 1);
        }
    }
    None
}

pub fn pop_class_stack_for_depth(class_stack: &mut Vec<(String, i32)>, depth: i32) {
    while class_stack.last().is_some_and(|(_, level)| depth < *level) {
        class_stack.pop();
    }
}

pub fn update_brace_depth(depth: &mut i32, line: &str) {
    *depth += line.chars().filter(|ch| *ch == '{').count() as i32;
    *depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
}

pub fn extract_jsdoc_comment_blocks(source: &str) -> Vec<DocCommentBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut idx = 0usize;

    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.starts_with("/**") {
            let start = idx + 1;
            let mut content = Vec::new();
            let mut end = start;
            while idx < lines.len() {
                let line = lines[idx].trim();
                let cleaned = line
                    .trim_start_matches("/**")
                    .trim_start_matches("*/")
                    .trim_start_matches('*')
                    .trim()
                    .to_string();
                if !cleaned.is_empty() {
                    content.push(cleaned);
                }
                if line.contains("*/") {
                    end = idx + 1;
                    break;
                }
                idx += 1;
            }
            blocks.push(DocCommentBlock {
                start_line: start,
                end_line: end,
                target_line_hint: next_nonempty_line(source, end),
                lines: content,
            });
        }
        idx += 1;
    }

    blocks
}

pub fn visibility_from_explicit_public(is_public: bool) -> DocVisibility {
    if is_public {
        DocVisibility::Public
    } else {
        DocVisibility::Private
    }
}

pub fn visibility_from_member_modifier(
    modifier: Option<&str>,
    default_visibility: DocVisibility,
) -> DocVisibility {
    match modifier.map(str::trim) {
        Some("private") => DocVisibility::Private,
        Some("protected") => DocVisibility::Protected,
        Some("public") => DocVisibility::Public,
        _ => default_visibility,
    }
}

pub fn visibility_from_leading_underscore(name: &str) -> DocVisibility {
    if name.starts_with('_') {
        DocVisibility::Private
    } else {
        DocVisibility::Public
    }
}

pub fn visibility_from_leading_uppercase(name: &str) -> DocVisibility {
    if name.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()) {
        DocVisibility::Public
    } else {
        DocVisibility::Private
    }
}

pub fn effective_member_visibility(
    declared_visibility: DocVisibility,
    container_visibility: Option<DocVisibility>,
    require_public_container: bool,
) -> DocVisibility {
    if require_public_container
        && container_visibility.is_some()
        && container_visibility != Some(DocVisibility::Public)
    {
        DocVisibility::Private
    } else {
        declared_visibility
    }
}

pub fn visibility_inherits_from_container(
    container_visibility: Option<DocVisibility>,
) -> DocVisibility {
    if container_visibility == Some(DocVisibility::Public) {
        DocVisibility::Public
    } else {
        DocVisibility::Private
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_helpers_cover_modifier_name_and_container_rules() {
        assert_eq!(visibility_from_explicit_public(true), DocVisibility::Public);
        assert_eq!(visibility_from_explicit_public(false), DocVisibility::Private);

        assert_eq!(
            visibility_from_member_modifier(Some("private"), DocVisibility::Public),
            DocVisibility::Private
        );
        assert_eq!(
            visibility_from_member_modifier(Some("protected"), DocVisibility::Public),
            DocVisibility::Protected
        );
        assert_eq!(
            visibility_from_member_modifier(Some("public"), DocVisibility::Private),
            DocVisibility::Public
        );
        assert_eq!(
            visibility_from_member_modifier(None, DocVisibility::Public),
            DocVisibility::Public
        );

        assert_eq!(visibility_from_leading_underscore("_helper"), DocVisibility::Private);
        assert_eq!(visibility_from_leading_underscore("api"), DocVisibility::Public);
        assert_eq!(visibility_from_leading_uppercase("Serve"), DocVisibility::Public);
        assert_eq!(visibility_from_leading_uppercase("serve"), DocVisibility::Private);

        assert_eq!(
            effective_member_visibility(DocVisibility::Public, Some(DocVisibility::Private), true),
            DocVisibility::Private
        );
        assert_eq!(
            effective_member_visibility(
                DocVisibility::Protected,
                Some(DocVisibility::Public),
                true
            ),
            DocVisibility::Protected
        );
        assert_eq!(
            effective_member_visibility(DocVisibility::Public, Some(DocVisibility::Private), false),
            DocVisibility::Public
        );

        assert_eq!(
            visibility_inherits_from_container(Some(DocVisibility::Public)),
            DocVisibility::Public
        );
        assert_eq!(
            visibility_inherits_from_container(Some(DocVisibility::Private)),
            DocVisibility::Private
        );
        assert_eq!(visibility_inherits_from_container(None), DocVisibility::Private);
    }
}
