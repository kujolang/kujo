use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect("failed to read doc")
}

#[test]
fn high_risk_docs_policies_remain_consistent() {
    let canonical =
        "Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.";

    let readme = read("README.md");
    let v1_scope = read("docs/V1_SCOPE.md");
    let lang_spec = read("docs/LANGUAGE_SPEC.md");
    let parity = read("docs/VM_INTERPRETER_PARITY_MATRIX.md");
    let migration = read("docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md");
    let stdlib_ref = read("docs/STANDARD_LIBRARY_REFERENCE.md");
    let architecture = read("docs/ARCHITECTURE.md");
    let ai_evidence = read("docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md");
    let official_release = read("docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md");
    let secure_ai = read("docs/SECURE_AI_SCRIPTING.md");
    let security_response = read("docs/SECURITY_RESPONSE.md");
    let hardening_status = read("docs/AI_NATIVE_PRODUCT_HARDENING_STATUS_2026-06-27.md");

    for (name, content) in [
        ("README.md", &readme),
        ("docs/V1_SCOPE.md", &v1_scope),
        ("docs/LANGUAGE_SPEC.md", &lang_spec),
        ("docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md", &ai_evidence),
        ("docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md", &official_release),
    ] {
        assert!(content.contains(canonical), "missing canonical readiness boundary in {}", name);
    }

    assert!(
        readme.contains(
            "Developers should not need `--interpreter` for ordinary modular project layouts."
        ),
        "README should document VM-first runtime recommendation for modular workflows"
    );
    assert!(
        readme.contains("kujo package-install --frozen"),
        "README should document package lockfile verification guidance"
    );
    assert!(
        parity.contains("Top-level generator iteration (`func*`, `yield`, `for ... in generator`)")
            && parity.contains("| supported |"),
        "VM/interpreter parity matrix should mark top-level generator iteration as supported"
    );
    assert!(
        parity.contains("kujo package-install --frozen"),
        "VM/interpreter parity matrix should document package workflow verification"
    );
    assert!(
        migration.contains("Package bootstrap and lockfile verification"),
        "migration playbook should call out package bootstrap and lockfile verification"
    );

    assert!(
        stdlib_ref.contains("`preview`: in-scope for v1 usage, but not frozen")
            && stdlib_ref.contains(
                "`experimental`: explicitly non-guaranteed for v1 compatibility commitments"
            ),
        "standard library tier policy should keep preview/experimental non-guarantee wording"
    );

    assert!(
        architecture.contains("VM (default `kujo run` path)")
            && architecture.contains("Tree-walking interpreter (explicit fallback path)"),
        "architecture doc should preserve VM-default and interpreter-fallback posture"
    );
    assert!(
        ai_evidence.contains("bash scripts/enterprise_verify.sh --full")
            && ai_evidence.contains("KUJO_AI_REPLAY_MODE=strict")
            && ai_evidence.contains("Do not set `KUJO_AI_REPLAY_MODE=fallthrough` in CI"),
        "AI-native evidence doc should keep enterprise verification and strict replay guidance"
    );
    assert!(
        secure_ai.contains("kujo run --untrusted --allow-ai script.kujo")
            && secure_ai
                .contains("Do not run deterministic AI test lanes with live provider credentials"),
        "secure AI scripting guide should keep least-privilege and no-live-CI guidance"
    );
    assert!(
        security_response.contains("Capability bypasses in `--untrusted` mode")
            && security_response.contains("AI egress allowlist bypasses")
            && security_response.contains("Secret redaction failures"),
        "security response doc should cover high-risk Kujo report classes"
    );
    assert!(
        hardening_status.contains("The core AI-native implementation track is complete")
            && hardening_status.contains("docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md"),
        "hardening status should keep the completed-track and remaining-readiness boundary"
    );
    assert!(
        official_release.contains("P0 runtime-parity-bug: 0")
            && official_release.contains("UNBLOCK_V1_RELEASE")
            && official_release.contains("bash scripts/release_gate.sh --full"),
        "official release checklist should keep current release blocker and verification commands"
    );
}
