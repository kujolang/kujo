//! Shared, immutable source-analysis result used by CLI and editor features.

use crate::ast::{Pattern, Stmt};
use crate::errors::KujoError;
use crate::lexer::{self, LexerDiagnostic, Token};
use crate::parser::{self, ParseDiagnostic};
use crate::type_checker::{AnalysisSnapshot, ModuleAnalysisCache, TypeChecker};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredSymbols {
    pub functions: BTreeSet<String>,
    pub variables: BTreeSet<String>,
    pub structs: BTreeSet<String>,
    pub imports: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub struct AnalyzedProgram {
    pub source: Arc<str>,
    pub tokens: Vec<Token>,
    #[allow(dead_code)] // Retained for consumers that need the shared parsed program.
    pub stmts: Vec<Stmt>,
    pub lexer_diagnostics: Vec<LexerDiagnostic>,
    pub parser_diagnostics: Vec<ParseDiagnostic>,
    pub semantic_diagnostics: Vec<KujoError>,
    pub facts: AnalysisSnapshot,
    pub declared: DeclaredSymbols,
}

impl AnalyzedProgram {
    pub fn analyze(
        source: impl Into<Arc<str>>,
        source_path: Option<&Path>,
        module_cache: Arc<ModuleAnalysisCache>,
    ) -> Self {
        let source = source.into();
        let lexed = lexer::tokenize_with_diagnostics(&source);
        let tokens = lexed.tokens;
        let mut parser = parser::Parser::new(tokens.clone());
        let parsed = parser.parse_with_diagnostics();
        let declared = collect_declared_symbols(&parsed.stmts);

        let search_paths = source_path
            .and_then(Path::parent)
            .map(|path| vec![path.to_path_buf()])
            .unwrap_or_default();
        let (facts, semantic_diagnostics) = if parsed.diagnostics.is_empty() {
            analyze_statements(&parsed.stmts, search_paths.iter(), module_cache)
        } else {
            (AnalysisSnapshot::default(), Vec::new())
        };

        Self {
            source,
            tokens,
            stmts: parsed.stmts,
            lexer_diagnostics: lexed.diagnostics,
            parser_diagnostics: parsed.diagnostics,
            semantic_diagnostics,
            facts,
            declared,
        }
    }
}

pub fn analyze_statements<'a>(
    stmts: &[Stmt],
    search_paths: impl IntoIterator<Item = &'a std::path::PathBuf>,
    module_cache: Arc<ModuleAnalysisCache>,
) -> (AnalysisSnapshot, Vec<KujoError>) {
    let mut checker = TypeChecker::with_module_cache(module_cache);
    for path in search_paths {
        checker.add_search_path(path);
    }
    let diagnostics = checker.check(stmts).err().unwrap_or_default();
    (checker.analysis_snapshot(), diagnostics)
}

fn collect_declared_symbols(stmts: &[Stmt]) -> DeclaredSymbols {
    let mut symbols = DeclaredSymbols::default();
    for stmt in stmts {
        collect_stmt(stmt, &mut symbols);
    }
    symbols
}

fn collect_stmt(stmt: &Stmt, symbols: &mut DeclaredSymbols) {
    match stmt {
        Stmt::Let { pattern, .. } => collect_pattern(pattern, &mut symbols.variables),
        Stmt::Const { name, .. } => {
            symbols.variables.insert(name.clone());
        }
        Stmt::FuncDef { name, body, .. } => {
            symbols.functions.insert(name.clone());
            for child in body {
                collect_stmt(child, symbols);
            }
        }
        Stmt::StructDef { name, methods, .. } => {
            symbols.structs.insert(name.clone());
            for method in methods {
                collect_stmt(method, symbols);
            }
        }
        Stmt::Import { module: _, symbols: Some(imported) } => {
            symbols.imports.extend(imported.iter().cloned());
        }
        Stmt::Import { module, symbols: None } => {
            symbols.imports.insert(crate::vm::VM::module_binding_name(module));
        }
        Stmt::For { var, body, .. } => {
            symbols.variables.insert(var.clone());
            for child in body {
                collect_stmt(child, symbols);
            }
        }
        Stmt::If { then_branch, else_branch, .. } => {
            for child in then_branch {
                collect_stmt(child, symbols);
            }
            if let Some(branch) = else_branch {
                for child in branch {
                    collect_stmt(child, symbols);
                }
            }
        }
        Stmt::Loop { body, .. }
        | Stmt::While { body, .. }
        | Stmt::Block(body)
        | Stmt::TestSetup { body }
        | Stmt::TestTeardown { body }
        | Stmt::Test { body, .. }
        | Stmt::Spawn { body } => {
            for child in body {
                collect_stmt(child, symbols);
            }
        }
        Stmt::TryExcept { try_block, except_var, except_block } => {
            symbols.variables.insert(except_var.clone());
            for child in try_block.iter().chain(except_block) {
                collect_stmt(child, symbols);
            }
        }
        Stmt::Export { stmt } => collect_stmt(stmt, symbols),
        Stmt::Match { cases, default, .. } => {
            for child in cases.iter().flat_map(|(_, body)| body) {
                collect_stmt(child, symbols);
            }
            if let Some(body) = default {
                for child in body {
                    collect_stmt(child, symbols);
                }
            }
        }
        Stmt::TestGroup { tests, .. } => {
            for child in tests {
                collect_stmt(child, symbols);
            }
        }
        _ => {}
    }
}

fn collect_pattern(pattern: &Pattern, variables: &mut BTreeSet<String>) {
    match pattern {
        Pattern::Identifier(name) if name != "_" => {
            variables.insert(name.clone());
        }
        Pattern::Array { elements, rest } => {
            for element in elements {
                collect_pattern(element, variables);
            }
            if let Some(name) = rest {
                variables.insert(name.clone());
            }
        }
        Pattern::Dict { keys, rest } => {
            variables.extend(keys.iter().cloned());
            if let Some(name) = rest {
                variables.insert(name.clone());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::AnalyzedProgram;
    use crate::ast::TypeAnnotation;
    use crate::type_checker::ModuleAnalysisCache;
    use std::fs;
    use std::sync::Arc;

    #[test]
    fn exposes_inferred_values_functions_and_structs() {
        let source = "struct User { name: string }\nfunc make_user() { return User { name: \"Ada\" } }\nlet user := make_user()\n";
        let analysis =
            AnalyzedProgram::analyze(source, None, Arc::new(ModuleAnalysisCache::default()));
        assert!(analysis.parser_diagnostics.is_empty(), "{:?}", analysis.parser_diagnostics);
        assert!(analysis.semantic_diagnostics.is_empty());
        assert_eq!(
            analysis.facts.variables.get("user"),
            Some(&TypeAnnotation::Struct("User".into())),
            "facts={:?}; stmts={:?}",
            analysis.facts,
            analysis.stmts
        );
        assert!(analysis.facts.functions.contains_key("make_user"));
        assert_eq!(analysis.facts.structs["User"].fields["name"], TypeAnnotation::String);
    }

    #[test]
    fn shared_module_cache_reuses_parses_and_invalidates_by_content() {
        let temp = tempfile::tempdir().unwrap();
        let module_path = temp.path().join("helpers.kujo");
        fs::write(&module_path, "export const answer := 1\n").unwrap();
        let main_path = temp.path().join("main.kujo");
        let cache = Arc::new(ModuleAnalysisCache::new(8));

        let first = AnalyzedProgram::analyze(
            "import helpers\nprint(helpers.answer)\n",
            Some(&main_path),
            Arc::clone(&cache),
        );
        assert!(first.semantic_diagnostics.is_empty());
        let first_stats = cache.stats();

        let second = AnalyzedProgram::analyze(
            "import helpers\nprint(helpers.answer)\n",
            Some(&main_path),
            Arc::clone(&cache),
        );
        assert!(second.semantic_diagnostics.is_empty());
        let second_stats = cache.stats();
        assert!(second_stats.hits > first_stats.hits);
        assert_eq!(second_stats.misses, first_stats.misses);

        fs::write(&module_path, "export const answer := 2\n").unwrap();
        let _ = AnalyzedProgram::analyze(
            "import helpers\nprint(helpers.answer)\n",
            Some(&main_path),
            Arc::clone(&cache),
        );
        assert!(cache.stats().misses > second_stats.misses);
    }
}
