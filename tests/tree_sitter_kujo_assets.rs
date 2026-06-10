use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn tree_sitter_kujo_assets_exist_and_have_core_content() {
    let grammar_path = root().join("tree-sitter-kujo/grammar.js");
    let corpus_path = root().join("tree-sitter-kujo/test/corpus/core.txt");
    let regressions_corpus_path = root().join("tree-sitter-kujo/test/corpus/regressions.txt");
    let highlights_path = root().join("tree-sitter-kujo/queries/highlights.scm");
    let injections_path = root().join("tree-sitter-kujo/queries/injections.scm");

    assert!(grammar_path.exists(), "missing grammar file");
    assert!(corpus_path.exists(), "missing corpus fixture");
    assert!(regressions_corpus_path.exists(), "missing regressions corpus fixture");
    assert!(highlights_path.exists(), "missing highlights query file");
    assert!(injections_path.exists(), "missing injections query file");

    let grammar = fs::read_to_string(grammar_path).expect("failed to read grammar.js");
    let corpus = fs::read_to_string(corpus_path).expect("failed to read corpus fixture");
    let regressions_corpus = fs::read_to_string(regressions_corpus_path)
        .expect("failed to read regressions corpus fixture");
    let highlights = fs::read_to_string(highlights_path).expect("failed to read highlights query");

    assert!(grammar.contains("name: 'kujo'"));
    assert!(grammar.contains("function_definition"));
    assert!(grammar.contains("variable_declaration"));

    assert!(corpus.contains("Function definition"));
    assert!(corpus.contains("Variable declaration and call"));

    assert!(regressions_corpus.contains("Async function and await call"));
    assert!(regressions_corpus.contains("Struct declaration and field access"));
    assert!(regressions_corpus.contains("Match with option patterns"));

    assert!(highlights.contains("@function"));
    assert!(highlights.contains("@variable"));
    assert!(highlights.contains("@keyword"));
    assert!(highlights.contains("\"async\" @keyword"));
    assert!(highlights.contains("\"await\" @keyword"));
    assert!(highlights.contains("\"struct\" @keyword"));
    assert!(highlights.contains("\"match\" @keyword"));
    assert!(highlights.contains("\"case\" @keyword"));
}
