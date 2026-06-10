use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vm_parity_doc_includes_command_level_runtime_path_matrix() {
    let matrix_path = repo_root().join("docs").join("VM_INTERPRETER_PARITY_MATRIX.md");
    let content = fs::read_to_string(&matrix_path)
        .expect("failed to read docs/VM_INTERPRETER_PARITY_MATRIX.md");

    for marker in [
        "## Command-Level Runtime Path Matrix",
        "| Command/Test Surface | Runtime Path | Alternate Path(s) | Why This Path Exists | Evidence |",
        "| `kujo run <file>` | VM (default) |",
        "| `kujo test --runtime dual` | VM-primary with bounded interpreter fallback |",
        "### `kujo test` Default Runtime Decision (2026-05-21)",
        "- Decision: keep default `kujo test` runtime at `dual` for now.",
        "## VM-First Practical Recommendations",
        "Treat `--interpreter` as an explicit compatibility/debug tool, not a default requirement for ordinary module-import workflows.",
        "| `kujo test-run <file>` | Interpreter-hosted test framework execution |",
        "| `cargo test --test native_api_security_boundaries` | Interpreter-focused command execution (`run --interpreter`) |",
        "| `cargo test --test diagnostics_golden` | Interpreter diagnostics command coverage (`run --interpreter`) |",
        "| `kujo lsp-diagnostics <file>` | Parse/diagnostic pipeline (runtime-agnostic) |",
        "| `kujo check <file>` | Parse/compile validation (runtime-agnostic) |",
    ] {
        assert!(
            content.contains(marker),
            "runtime-path matrix should contain marker {:?}",
            marker
        );
    }
}
