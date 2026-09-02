use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn readme_covers_v1_status_cli_security_and_core_reference_links() {
    let readme_path = repo_root().join("README.md");
    let content = fs::read_to_string(&readme_path).expect("failed to read README.md");

    let required_markers = [
        "## 1.0 Release Status",
        "## Safety Model Snapshot",
        "## AI-Native Runtime Snapshot",
        "## Core Reference Links",
        "This first-ten-minutes path gives you a normal script, a replay-only AI example",
        "cargo build --release",
        "kujo run hello.kujo",
        "examples/ai_enterprise_replay_showcase.kujo",
        "KUJO_AI_REPLAY_MODE=strict",
        "bash scripts/enterprise_verify.sh --minimal",
        "bash scripts/enterprise_verify.sh --full",
        "kujo serve [dir]",
        "--untrusted",
        "[ROADMAP.md](ROADMAP.md)",
        "[docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md)",
        "[docs/STANDARD_LIBRARY.md](docs/STANDARD_LIBRARY.md)",
        "[docs/AI_RUNTIME.md](docs/AI_RUNTIME.md)",
        "[docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md](docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md)",
        "[docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md)",
        "[docs/SECURE_AI_SCRIPTING.md](docs/SECURE_AI_SCRIPTING.md)",
        "[docs/SECURITY_RESPONSE.md](docs/SECURITY_RESPONSE.md)",
        "[docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md)",
        "Core AI-native runtime mechanisms are implemented for deterministic request hashing, offline record/replay cassettes, structured response metadata, JSON Schema validation, vector math, token budgeting, runtime secret redaction, dedicated AI egress capability controls, streaming callbacks, and multimodal message builders.",
        "Release boundary: Kujo `v1.2.3` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.",
        "ai_text`, `ai_image_url`, and `ai_message",
        "Dotted module import workflows are supported on the default VM path",
        "Package workflows are deterministic: `kujo init`, `kujo package-add`, `kujo package-install`, and `kujo package-install --frozen` work with nested source layouts and reproducible `kujo.lock` snapshots.",
        "Kujo v1.0 package scope is local manifest and lockfile determinism only; it does not include a public Kennel registry or package publish transport.",
        "## Runtime Mode Recommendations",
        "Developers should not need `--interpreter` for ordinary modular project layouts.",
        "kujo package-install --frozen",
        "`kujo package-publish`: preview package publish metadata only",
        "[docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md](docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md)",
    ];

    for marker in required_markers {
        assert!(content.contains(marker), "expected README to include marker {marker:?}");
    }
}
