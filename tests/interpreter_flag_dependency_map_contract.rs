use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("kujo_{label}_{nonce}"));
    fs::create_dir_all(&dir).expect("failed to create temp directory");
    dir
}

#[test]
fn interpreter_flag_dependency_map_generator_covers_required_surfaces_and_tags() {
    let root = repo_root();
    let temp_dir = unique_temp_dir("interpreter_flag_map");
    let output_path = temp_dir.join("INTERPRETER_FLAG_DEPENDENCY_MAP.md");

    let output = Command::new("bash")
        .current_dir(&root)
        .args([
            "scripts/generate_interpreter_flag_dependency_map.sh",
            output_path.to_str().expect("path should be utf-8"),
        ])
        .output()
        .expect("failed to run interpreter dependency map generator");

    assert!(
        output.status.success(),
        "map generator should succeed, status={:?}, stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let map = fs::read_to_string(output_path).expect("expected generated dependency map");
    for marker in [
        "# Interpreter Flag Dependency Map",
        "Reason tags:",
        "`harness-legacy`",
        "`parity-gap`",
        "`security-test-choice`",
        "`docs-contract`",
        "| File | Category | Reason Tags | Usage Count | Line References |",
        "## V1U-RUN-005: Parity-Gap Coverage Status",
        "Current `parity-gap` tagged entries:",
        "## V1U-RUN-002: `kujo test` Runtime Strategy Status",
        "`kujo test` supports explicit runtime strategy selection via `--runtime dual|vm|interpreter`",
        "Import-reliability clarification:",
        "`--interpreter` is not required for ordinary multi-module import layouts",
        "VM-first practical recommendations:",
        "Use `kujo run <file>` as the default VM-first path for ordinary modular projects.",
    ] {
        assert!(map.contains(marker), "dependency map should contain required marker {marker:?}");
    }

    for required_row in [
        "| `src/parser.rs` | cli-harness | `harness-legacy,parity-gap` |",
        "| `tests/native_api_security_boundaries.rs` | integration-test | `security-test-choice` |",
        "| `tests/docs_examples.rs` | integration-test | `docs-smoke,harness-legacy` |",
        "| `README.md` | documentation | `docs-contract` |",
    ] {
        assert!(
            map.contains(required_row),
            "dependency map should contain required row fragment {required_row:?}"
        );
    }
}

#[test]
fn parser_test_harness_keeps_explicit_interpreter_fallback_path() {
    let root = repo_root();
    let parser_path = root.join("src").join("parser.rs");
    let parser_source = fs::read_to_string(parser_path).expect("expected src/parser.rs source");

    assert!(
        parser_source.contains("pub fn run_all_tests"),
        "parser source should define run_all_tests harness entrypoint"
    );
    assert!(
        parser_source.contains(".arg(\"--interpreter\")"),
        "run_all_tests should keep an explicit interpreter execution path for bounded fallback coverage"
    );
}
