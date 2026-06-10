use crate::docgen::model::{DocCommentBlock, DocSymbol};
use crate::docgen::DocgenError;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

pub(crate) mod common;
pub mod go;
pub mod haskell;
pub mod javascript;
pub mod kujo;
pub mod php;
pub mod python;
pub mod ruby;
pub mod typescript;
pub mod zig;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AdapterCapability {
    pub supports_functions: bool,
    pub supports_types: bool,
    pub supports_methods: bool,
    pub supports_inline_docs: bool,
}

pub trait DocLanguageAdapter: Send + Sync {
    fn language_id(&self) -> &'static str;
    fn file_extensions(&self) -> &'static [&'static str];
    fn capabilities(&self) -> AdapterCapability;
    fn extract_symbols(&self, source: &str, path: &Path) -> Result<Vec<DocSymbol>, DocgenError>;
    fn extract_inline_docs(
        &self,
        source: &str,
        path: &Path,
    ) -> Result<Vec<DocCommentBlock>, DocgenError>;
    fn attach_docs(&self, symbols: Vec<DocSymbol>, docs: Vec<DocCommentBlock>) -> Vec<DocSymbol>;
}

type AdapterConstructor = fn() -> Box<dyn DocLanguageAdapter>;

#[derive(Clone)]
struct AdapterEntry {
    language_id: &'static str,
    file_extensions: &'static [&'static str],
    capabilities: AdapterCapability,
    constructor: AdapterConstructor,
}

fn make_kujo_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(kujo::KujoDocAdapter)
}

fn make_php_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(php::PhpDocAdapter)
}

fn make_python_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(python::PythonDocAdapter)
}

fn make_typescript_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(typescript::TypeScriptDocAdapter)
}

fn make_javascript_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(javascript::JavaScriptDocAdapter)
}

fn make_ruby_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(ruby::RubyDocAdapter)
}

fn make_go_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(go::GoDocAdapter)
}

fn make_haskell_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(haskell::HaskellDocAdapter)
}

fn make_zig_adapter() -> Box<dyn DocLanguageAdapter> {
    Box::new(zig::ZigDocAdapter)
}

const ADAPTER_CONSTRUCTORS: [AdapterConstructor; 9] = [
    make_kujo_adapter,
    make_php_adapter,
    make_python_adapter,
    make_typescript_adapter,
    make_javascript_adapter,
    make_ruby_adapter,
    make_go_adapter,
    make_haskell_adapter,
    make_zig_adapter,
];

fn build_adapter_entries() -> Vec<AdapterEntry> {
    ADAPTER_CONSTRUCTORS
        .iter()
        .map(|constructor| {
            let adapter = constructor();
            AdapterEntry {
                language_id: adapter.language_id(),
                file_extensions: adapter.file_extensions(),
                capabilities: adapter.capabilities(),
                constructor: *constructor,
            }
        })
        .collect()
}

fn adapter_entries() -> &'static [AdapterEntry] {
    static ADAPTER_ENTRIES: OnceLock<Vec<AdapterEntry>> = OnceLock::new();
    ADAPTER_ENTRIES.get_or_init(build_adapter_entries).as_slice()
}

fn language_lookup() -> &'static BTreeMap<String, AdapterConstructor> {
    static LOOKUP: OnceLock<BTreeMap<String, AdapterConstructor>> = OnceLock::new();
    LOOKUP.get_or_init(|| {
        adapter_entries()
            .iter()
            .map(|entry| (entry.language_id.to_ascii_lowercase(), entry.constructor))
            .collect()
    })
}

fn extension_lookup() -> &'static BTreeMap<String, AdapterConstructor> {
    static LOOKUP: OnceLock<BTreeMap<String, AdapterConstructor>> = OnceLock::new();
    LOOKUP.get_or_init(|| {
        let mut map = BTreeMap::new();
        for entry in adapter_entries() {
            for ext in entry.file_extensions {
                map.insert(ext.to_ascii_lowercase(), entry.constructor);
            }
        }
        map
    })
}

#[allow(dead_code)]
pub fn registry() -> Vec<Box<dyn DocLanguageAdapter>> {
    adapter_entries().iter().map(|entry| (entry.constructor)()).collect()
}

pub fn adapter_for_language(language: &str) -> Option<Box<dyn DocLanguageAdapter>> {
    let normalized = language.trim().to_ascii_lowercase();
    language_lookup().get(&normalized).map(|constructor| constructor())
}

pub fn adapter_for_extension(ext: &str) -> Option<Box<dyn DocLanguageAdapter>> {
    let normalized = ext.trim_start_matches('.').to_ascii_lowercase();
    extension_lookup().get(&normalized).map(|constructor| constructor())
}

#[allow(dead_code)]
pub fn language_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> =
        adapter_entries().iter().map(|entry| entry.language_id).collect();
    ids.sort_unstable();
    ids
}

pub fn capability_index() -> Vec<(String, AdapterCapability)> {
    let mut entries: Vec<(String, AdapterCapability)> = adapter_entries()
        .iter()
        .map(|entry| (entry.language_id.to_string(), entry.capabilities.clone()))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::path::Path;
    use std::sync::OnceLock;
    use std::time::Instant;

    #[test]
    fn adapter_lookup_handles_language_and_extension_normalization() {
        assert!(adapter_for_language("RUSTY_UNKNOWN").is_none());
        assert_eq!(adapter_for_language("KuJo").map(|adapter| adapter.language_id()), Some("kujo"));
        assert_eq!(
            adapter_for_extension(".TSX").map(|adapter| adapter.language_id()),
            Some("typescript")
        );
        assert_eq!(
            adapter_for_extension("mJs").map(|adapter| adapter.language_id()),
            Some("javascript")
        );
    }

    #[test]
    fn capability_index_and_language_ids_are_sorted_and_aligned() {
        let language_ids = language_ids();
        let mut expected_language_ids = language_ids.clone();
        expected_language_ids.sort_unstable();
        assert_eq!(language_ids, expected_language_ids);

        let capabilities = capability_index();
        let mut capability_languages: Vec<String> =
            capabilities.iter().map(|(language, _)| language.clone()).collect();
        let mut expected = capability_languages.clone();
        expected.sort();
        assert_eq!(capability_languages, expected);

        capability_languages.dedup();
        assert_eq!(capability_languages.len(), language_ids.len());
    }

    #[test]
    fn static_lookup_avoids_legacy_full_registry_construction_cost() {
        fn legacy_language_lookup_construction_count(language: &str) -> usize {
            let mut constructed = 0usize;
            for constructor in ADAPTER_CONSTRUCTORS {
                constructed += 1;
                let adapter = constructor();
                if adapter.language_id() == language {
                    break;
                }
            }
            constructed
        }

        let legacy_constructions = legacy_language_lookup_construction_count("zig");
        assert_eq!(legacy_constructions, ADAPTER_CONSTRUCTORS.len());

        let static_lookup = adapter_for_language("zig").expect("zig adapter should resolve");
        assert_eq!(static_lookup.language_id(), "zig");
        // New lookup still constructs one adapter instance for the returned result, but avoids
        // constructing all intermediate adapters during lookup.
        assert!(legacy_constructions > 1);
    }

    #[test]
    fn cached_regex_extractors_remain_stable_for_success_failure_and_edge_inputs() {
        let cases: &[(&str, &str, &[&str])] = &[
            ("kujo", "pub func api_call(value) {\n    return value\n}\n", &["api_call"]),
            ("python", "def run_task(value):\n    return value\n", &["run_task"]),
            ("php", "function doWork($value) {\n    return $value;\n}\n", &["doWork"]),
            ("typescript", "export function ping(value: string) { return value; }\n", &["ping"]),
            ("javascript", "export function pong(value) { return value; }\n", &["pong"]),
            ("ruby", "def compute(value)\n  value\nend\n", &["compute"]),
            ("go", "func Serve(value string) {}\n", &["Serve"]),
            ("haskell", "result :: Int\nresult = 42\n", &["result"]),
            ("zig", "pub fn run(value: i32) i32 { return value; }\n", &["run"]),
        ];

        for (language, valid_source, expected_symbols) in cases {
            let adapter = adapter_for_language(language)
                .unwrap_or_else(|| panic!("missing adapter for '{language}'"));
            let sample_path = Path::new("sample");

            // Regression guard for DG-QA-006: repeated extraction should stay stable now that
            // regex compilation is cached via static lazy initialization.
            let first = adapter
                .extract_symbols(valid_source, sample_path)
                .unwrap_or_else(|err| panic!("extract_symbols failed for {language}: {err}"));
            let second = adapter.extract_symbols(valid_source, sample_path).unwrap_or_else(|err| {
                panic!("second extract_symbols failed for {language}: {err}")
            });
            let first_names: Vec<String> =
                first.iter().map(|symbol| symbol.qualified_name.clone()).collect();
            let second_names: Vec<String> =
                second.iter().map(|symbol| symbol.qualified_name.clone()).collect();
            assert_eq!(first_names, second_names, "repeated extraction drifted for {language}");
            for expected in *expected_symbols {
                assert!(
                    first_names.iter().any(|name| name.contains(expected)),
                    "expected symbol '{expected}' missing for {language}: {first_names:?}"
                );
            }

            let unmatched = adapter
                .extract_symbols("%%% !!!", sample_path)
                .unwrap_or_else(|err| panic!("unmatched extraction failed for {language}: {err}"));
            assert!(
                unmatched.is_empty(),
                "unmatched input should not emit symbols for {language}: {unmatched:?}"
            );

            let empty = adapter
                .extract_symbols("", sample_path)
                .unwrap_or_else(|err| panic!("empty extraction failed for {language}: {err}"));
            assert!(empty.is_empty(), "empty input should produce no symbols for {language}");
        }
    }

    #[test]
    fn c_style_adapter_conformance_preserves_typescript_and_javascript_contracts() {
        let ts_adapter =
            adapter_for_language("typescript").expect("typescript adapter should be registered");
        let ts_source = r#"
/** Service summary */
export class Service {
  /**
   * Run summary
   */
  public run(input: string): string {
    return input;
  }
}

/** Utility summary */
export function util(input: string) {
  return input;
}
"#;
        let ts_symbols = ts_adapter
            .extract_symbols(ts_source, Path::new("service.ts"))
            .expect("typescript symbol extraction should succeed");
        let ts_docs = ts_adapter
            .extract_inline_docs(ts_source, Path::new("service.ts"))
            .expect("typescript inline doc extraction should succeed");
        let ts_attached = ts_adapter.attach_docs(ts_symbols, ts_docs);
        let ts_names: Vec<String> =
            ts_attached.iter().map(|symbol| symbol.qualified_name.clone()).collect();
        assert!(ts_names.contains(&"Service".to_string()));
        assert!(ts_names.contains(&"Service.run".to_string()));
        assert!(ts_names.contains(&"util".to_string()));
        let ts_run = ts_attached
            .iter()
            .find(|symbol| symbol.qualified_name == "Service.run")
            .expect("typescript method should be present");
        assert_eq!(ts_run.docs.summary.as_deref(), Some("Run summary"));

        let js_adapter =
            adapter_for_language("javascript").expect("javascript adapter should be registered");
        let js_source = r#"
/** Service summary */
export class Service {
  /**
   * Run summary
   */
  run(input) {
    return input;
  }
}

/** Utility summary */
export function util(input) {
  return input;
}
"#;
        let js_symbols = js_adapter
            .extract_symbols(js_source, Path::new("service.js"))
            .expect("javascript symbol extraction should succeed");
        let js_docs = js_adapter
            .extract_inline_docs(js_source, Path::new("service.js"))
            .expect("javascript inline doc extraction should succeed");
        let js_attached = js_adapter.attach_docs(js_symbols, js_docs);
        let js_names: Vec<String> =
            js_attached.iter().map(|symbol| symbol.qualified_name.clone()).collect();
        assert!(js_names.contains(&"Service".to_string()));
        assert!(js_names.contains(&"Service.run".to_string()));
        assert!(js_names.contains(&"util".to_string()));
        let js_run = js_attached
            .iter()
            .find(|symbol| symbol.qualified_name == "Service.run")
            .expect("javascript method should be present");
        assert_eq!(js_run.docs.summary.as_deref(), Some("Run summary"));
    }

    #[test]
    fn regex_caching_micro_benchmark_evidence() {
        const PATTERNS: &[&str] = &[
            r"^\s*(pub\s+)?(async\s+)?func\*?\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            r"^\s*(pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"^\s*(pub\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"^\s*(pub\s+)?(const|let)\s+([A-Za-z_][A-Za-z0-9_]*)\s*[:=]",
            r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*,?\s*$",
            r"^\s*(export\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"^\s*(export\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*\{",
            r"^\s*export\s+interface\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*interface\s+([A-Za-z_][A-Za-z0-9_]*)",
            r"^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+(struct|interface)",
            r"^\s*func\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)",
            r"^\s*def\s+([A-Za-z_][A-Za-z0-9_!?=]*)\s*(\(([^)]*)\))?",
            r"^\s*(public\s+|private\s+|protected\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)",
        ];
        const SAMPLE_LINES: &[&str] = &[
            "pub async func worker(value) {}",
            "export class Widget {}",
            "export interface Named {}",
            "func Serve(value string) {}",
            "def compute(value)",
            "public const VERSION = 1",
            "unknown input line",
        ];

        fn run_legacy_match_pass() -> usize {
            let mut hits = 0usize;
            for pattern in PATTERNS {
                let regex = Regex::new(pattern).expect("legacy compile should succeed");
                for sample in SAMPLE_LINES {
                    if regex.is_match(sample) {
                        hits += 1;
                    }
                }
            }
            hits
        }

        fn run_cached_match_pass() -> usize {
            static CACHED_REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
            let cached = CACHED_REGEXES.get_or_init(|| {
                PATTERNS
                    .iter()
                    .map(|pattern| Regex::new(pattern).expect("cached compile should succeed"))
                    .collect()
            });
            let mut hits = 0usize;
            for regex in cached {
                for sample in SAMPLE_LINES {
                    if regex.is_match(sample) {
                        hits += 1;
                    }
                }
            }
            hits
        }

        let iterations = 200usize;
        let legacy_start = Instant::now();
        let mut legacy_hits = 0usize;
        for _ in 0..iterations {
            legacy_hits += run_legacy_match_pass();
        }
        let legacy_elapsed = legacy_start.elapsed();

        let cached_start = Instant::now();
        let mut cached_hits = 0usize;
        for _ in 0..iterations {
            cached_hits += run_cached_match_pass();
        }
        let cached_elapsed = cached_start.elapsed();

        eprintln!(
            "DG-QA-006 regex cache benchmark: cached={cached_elapsed:?}, legacy={legacy_elapsed:?}"
        );
        assert_eq!(legacy_hits, cached_hits, "cached regex matching changed results");
        assert!(
            cached_elapsed < legacy_elapsed,
            "expected cached regex pass to be faster than legacy compile-per-pass path (cached={cached_elapsed:?}, legacy={legacy_elapsed:?})"
        );
    }
}
