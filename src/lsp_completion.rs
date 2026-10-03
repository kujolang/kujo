use crate::analyzed_program::AnalyzedProgram;
use crate::interpreter::Interpreter;
use crate::type_checker::ModuleAnalysisCache;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    Builtin,
    Function,
    Variable,
    Struct,
    Field,
    Method,
    Module,
}

impl CompletionItemKind {
    fn rank(&self) -> u8 {
        match self {
            CompletionItemKind::Builtin => 1,
            CompletionItemKind::Function => 2,
            CompletionItemKind::Variable => 3,
            CompletionItemKind::Struct => 4,
            CompletionItemKind::Field => 5,
            CompletionItemKind::Method => 6,
            CompletionItemKind::Module => 4,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CompletionItemKind::Builtin => "builtin",
            CompletionItemKind::Function => "function",
            CompletionItemKind::Variable => "variable",
            CompletionItemKind::Struct => "struct",
            CompletionItemKind::Field => "field",
            CompletionItemKind::Method => "method",
            CompletionItemKind::Module => "module",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
}

pub fn complete(source: &str, line: usize, column: usize) -> Vec<CompletionItem> {
    let analysis = AnalyzedProgram::analyze(source, None, Arc::new(ModuleAnalysisCache::default()));
    complete_with_analysis(&analysis, line, column)
}

pub fn complete_with_analysis(
    analysis: &AnalyzedProgram,
    line: usize,
    column: usize,
) -> Vec<CompletionItem> {
    let source = &analysis.source;
    let prefix = identifier_prefix_before_cursor(source, line, column);
    let mut by_label: BTreeMap<String, (CompletionItemKind, Option<String>)> = BTreeMap::new();

    if let Some(receiver) = member_receiver_before_cursor(source, line, column) {
        if let Some(crate::ast::TypeAnnotation::Struct(struct_name)) =
            analysis.facts.variables.get(&receiver)
        {
            if let Some(shape) = analysis.facts.structs.get(struct_name) {
                for (name, ty) in &shape.fields {
                    if name.starts_with(&prefix) {
                        by_label.insert(
                            name.clone(),
                            (CompletionItemKind::Field, Some(format!("{}: {}", name, ty))),
                        );
                    }
                }
                for (name, signature) in &shape.methods {
                    if name.starts_with(&prefix) {
                        by_label.insert(
                            name.clone(),
                            (CompletionItemKind::Method, Some(signature.display_signature(name))),
                        );
                    }
                }
            }
        } else if let Some(module) = analysis.facts.modules.get(&receiver) {
            for (name, ty) in &module.values {
                if name.starts_with(&prefix) {
                    by_label.insert(
                        name.clone(),
                        (CompletionItemKind::Variable, Some(format!("{}: {}", name, ty))),
                    );
                }
            }
            for (name, signature) in &module.functions {
                if name.starts_with(&prefix) {
                    by_label.insert(
                        name.clone(),
                        (CompletionItemKind::Function, Some(signature.display_signature(name))),
                    );
                }
            }
            for name in module.structs.keys() {
                if name.starts_with(&prefix) {
                    by_label.insert(
                        name.clone(),
                        (CompletionItemKind::Struct, Some(format!("struct {}", name))),
                    );
                }
            }
        }
        return by_label
            .into_iter()
            .map(|(label, (kind, detail))| CompletionItem { label, kind, detail })
            .collect();
    }

    for builtin in Interpreter::get_builtin_names() {
        upsert_completion_item(
            &mut by_label,
            builtin.to_string(),
            CompletionItemKind::Builtin,
            None,
        );
    }

    for function_name in &analysis.declared.functions {
        let detail = analysis
            .facts
            .functions
            .get(function_name)
            .map(|signature| signature.display_signature(function_name));
        upsert_completion_item(
            &mut by_label,
            function_name.clone(),
            CompletionItemKind::Function,
            detail,
        );
    }
    for variable_name in &analysis.declared.variables {
        let detail = analysis
            .facts
            .variables
            .get(variable_name)
            .map(|ty| format!("{}: {}", variable_name, ty));
        upsert_completion_item(
            &mut by_label,
            variable_name.clone(),
            CompletionItemKind::Variable,
            detail,
        );
    }
    for struct_name in &analysis.declared.structs {
        upsert_completion_item(
            &mut by_label,
            struct_name.clone(),
            CompletionItemKind::Struct,
            Some(format!("struct {}", struct_name)),
        );
    }
    for name in &analysis.declared.imports {
        if let Some(signature) = analysis.facts.functions.get(name) {
            upsert_completion_item(
                &mut by_label,
                name.clone(),
                CompletionItemKind::Function,
                Some(signature.display_signature(name)),
            );
        } else if analysis.facts.structs.contains_key(name) {
            upsert_completion_item(
                &mut by_label,
                name.clone(),
                CompletionItemKind::Struct,
                Some(format!("struct {}", name)),
            );
        } else if let Some(ty) = analysis.facts.variables.get(name) {
            let kind = if matches!(ty, crate::ast::TypeAnnotation::Module(_)) {
                CompletionItemKind::Module
            } else {
                CompletionItemKind::Variable
            };
            upsert_completion_item(
                &mut by_label,
                name.clone(),
                kind,
                Some(format!("{}: {}", name, ty)),
            );
        }
    }
    for (name, ty) in &analysis.facts.variables {
        if matches!(ty, crate::ast::TypeAnnotation::Module(_)) {
            upsert_completion_item(
                &mut by_label,
                name.clone(),
                CompletionItemKind::Module,
                Some(ty.to_string()),
            );
        }
    }

    by_label
        .into_iter()
        .filter(|(label, _)| label.starts_with(&prefix))
        .map(|(label, (kind, detail))| CompletionItem { label, kind, detail })
        .collect()
}

fn upsert_completion_item(
    by_label: &mut BTreeMap<String, (CompletionItemKind, Option<String>)>,
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
) {
    match by_label.get(&label) {
        Some((existing_kind, _)) if existing_kind.rank() >= kind.rank() => {}
        _ => {
            by_label.insert(label, (kind, detail));
        }
    }
}

fn member_receiver_before_cursor(source: &str, line: usize, column: usize) -> Option<String> {
    let selected_line = source.lines().nth(line.checked_sub(1)?)?;
    let prefix: String = selected_line.chars().take(column.saturating_sub(1)).collect();
    let before_member = prefix.rsplit_once('.')?.0.trim_end();
    let receiver = before_member
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    (!receiver.is_empty()).then_some(receiver)
}

fn identifier_prefix_before_cursor(source: &str, line: usize, column: usize) -> String {
    if line == 0 || column == 0 {
        return String::new();
    }

    let selected_line = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let safe_prefix_char_count = column.saturating_sub(1).min(selected_line.chars().count());

    let line_prefix: String = selected_line.chars().take(safe_prefix_char_count).collect();
    let mut prefix_chars = Vec::new();

    for character in line_prefix.chars().rev() {
        if character.is_ascii_alphanumeric() || character == '_' {
            prefix_chars.push(character);
        } else {
            break;
        }
    }

    prefix_chars.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::{
        complete, complete_with_analysis, identifier_prefix_before_cursor, CompletionItemKind,
    };
    use crate::analyzed_program::AnalyzedProgram;
    use crate::type_checker::ModuleAnalysisCache;
    use std::fs;
    use std::sync::Arc;

    #[test]
    fn identifier_prefix_tracks_word_before_cursor() {
        let source = "let value := 1\npri\n";
        assert_eq!(identifier_prefix_before_cursor(source, 2, 4), "pri");
        assert_eq!(identifier_prefix_before_cursor(source, 2, 3), "pr");
        assert_eq!(identifier_prefix_before_cursor(source, 2, 1), "");
    }

    #[test]
    fn completion_includes_builtin_function_and_variable_matches() {
        let source = [
            "func compute_total(x) {",
            "    return x",
            "}",
            "let printer := 1",
            "let project_name := \"kujo\"",
            "pr",
            "co",
        ]
        .join("\n");

        let completions = complete(&source, 6, 3);
        let completion_pairs: Vec<(String, CompletionItemKind)> =
            completions.iter().map(|item| (item.label.clone(), item.kind.clone())).collect();

        assert!(completion_pairs.contains(&("print".to_string(), CompletionItemKind::Builtin)));
        assert!(completion_pairs.contains(&("printer".to_string(), CompletionItemKind::Variable)));
        assert!(
            completion_pairs.contains(&("project_name".to_string(), CompletionItemKind::Variable))
        );

        let function_completions = complete(&source, 6, 3);
        assert!(!function_completions.iter().any(|item| item.label == "compute_total"));

        let co_completions = complete(&source, 7, 3);
        assert!(co_completions.iter().any(|item| item.label == "compute_total"));
    }

    #[test]
    fn member_completion_uses_inferred_struct_shape() {
        let source = "struct User { name: string, age: int }\nlet user := User { name: \"Ada\", age: 37 }\nuser.na";
        let items = complete(source, 3, 8);
        let name = items.iter().find(|item| item.label == "name").expect("field completion");
        assert_eq!(name.kind, CompletionItemKind::Field);
        assert_eq!(name.detail.as_deref(), Some("name: String"));
    }

    #[test]
    fn completion_surfaces_imported_signature_and_namespace_members() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("helpers.kujo"),
            "export func greet(name: string) -> string { return name }\n",
        )
        .unwrap();
        let source = "import helpers\nhelpers.gr";
        let analysis = AnalyzedProgram::analyze(
            source,
            Some(&temp.path().join("main.kujo")),
            Arc::new(ModuleAnalysisCache::default()),
        );
        let items = complete_with_analysis(&analysis, 2, 11);
        let greet = items.iter().find(|item| item.label == "greet").expect("imported function");
        assert_eq!(greet.kind, CompletionItemKind::Function);
        assert!(greet.detail.as_deref().unwrap_or_default().contains("String"));
    }

    #[test]
    fn completion_prefers_user_defined_symbol_kind_on_name_collision() {
        let source = "func print() { return null }\npr\n";
        let completions = complete(source, 2, 3);

        let print_item = completions
            .iter()
            .find(|item| item.label == "print")
            .expect("expected print completion to exist");
        assert_eq!(print_item.kind, CompletionItemKind::Function);
    }
}
