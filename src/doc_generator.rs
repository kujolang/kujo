#![allow(dead_code)]

use crate::docgen::adapters::kujo::KujoDocAdapter;
use crate::docgen::adapters::DocLanguageAdapter;
use crate::docgen::core::{run as run_docgen, DocOutputFormat, DocgenConfig};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct DocItem {
    pub name: String,
    pub line: usize,
    pub docs: Vec<String>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocGenerationSummary {
    pub output_dir: PathBuf,
    pub module_doc_path: PathBuf,
    pub builtin_doc_path: Option<PathBuf>,
    pub item_count: usize,
}

pub fn generate_docs_for_file(
    source_path: &Path,
    output_dir: &Path,
    include_builtins: bool,
) -> Result<DocGenerationSummary, String> {
    let (_project, summary) = run_docgen(&DocgenConfig {
        input: source_path.to_path_buf(),
        out_dir: output_dir.to_path_buf(),
        format: DocOutputFormat::Html,
        include_builtins,
        language: Some("kujo".to_string()),
        languages: None,
        emit_ai_tasks: false,
        search_index: false,
        source_links: true,
        source_link_template: None,
        fail_on_undocumented: false,
        fail_on_broken_links: false,
        fail_on_warnings: false,
        public_only: false,
        include_private: true,
        max_discovery_file_size_bytes: None,
        max_discovery_files: None,
        max_discovery_depth: None,
        cache_dir: None,
    })
    .map_err(|err| err.to_string())?;

    Ok(DocGenerationSummary {
        output_dir: summary.output_dir,
        module_doc_path: summary.module_doc_path,
        builtin_doc_path: summary.builtin_doc_path,
        item_count: summary.item_count,
    })
}

pub fn extract_doc_items(source: &str) -> Vec<DocItem> {
    let adapter = KujoDocAdapter;
    let path = Path::new("inline.kujo");
    let Ok(symbols) = adapter.extract_symbols(source, path) else {
        return Vec::new();
    };
    let Ok(docs) = adapter.extract_inline_docs(source, path) else {
        return Vec::new();
    };

    adapter
        .attach_docs(symbols, docs)
        .into_iter()
        .filter(|symbol| {
            !symbol.docs.placeholder
                && matches!(symbol.kind, crate::docgen::model::DocSymbolKind::Function)
        })
        .map(|symbol| DocItem {
            name: symbol.name,
            line: symbol.line,
            docs: symbol.docs.lines,
            examples: symbol.examples.into_iter().map(|example| example.code).collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{extract_doc_items, generate_docs_for_file};
    use std::fs;

    #[test]
    fn extract_doc_items_reads_comments_and_examples() {
        let source = "/// Adds two numbers\n/// ```kujo\n/// add(1, 2)\n/// ```\nfunc add(a, b) {\n    return a + b\n}\n";

        let items = extract_doc_items(source);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "add");
        assert!(items[0].docs.iter().any(|line| line.contains("Adds two numbers")));
        assert_eq!(items[0].examples.len(), 1);
        assert!(items[0].examples[0].contains("add(1, 2)"));
    }

    #[test]
    fn generate_docs_writes_module_and_builtin_pages() {
        let source = "/// Echo value\nfunc echo(value) {\n    return value\n}\n";

        let unique = format!(
            "kujo_docgen_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be valid")
                .as_nanos()
        );

        let base_dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base_dir).expect("temp output dir should be created");

        let source_path = base_dir.join("module.kujo");
        fs::write(&source_path, source).expect("source file should be written");

        let output_dir = base_dir.join("docs");
        let summary = generate_docs_for_file(&source_path, &output_dir, true)
            .expect("doc generation should succeed");

        assert!(summary.item_count >= 1);
        assert!(summary.module_doc_path.exists());
        assert!(summary.builtin_doc_path.as_ref().map(|path| path.exists()).unwrap_or(false));
        assert!(output_dir.join("index.html").exists());
    }
}
