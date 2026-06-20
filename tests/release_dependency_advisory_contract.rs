use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|err| panic!("failed to read {path}: {err}"))
}

#[test]
fn lockfile_no_longer_contains_resolved_advisory_warning_crates() {
    let lockfile = read("Cargo.lock");

    for removed in ["name = \"core2\"", "name = \"proc-macro-error2\""] {
        assert!(
            !lockfile.contains(removed),
            "Cargo.lock should not reintroduce resolved advisory warning crate {removed:?}"
        );
    }
}

#[test]
fn next_session_doc_lists_only_release_blockers_and_external_dependency_deferrals() {
    let handoff = read("docs/V1_0_NEXT_SESSION_ACTIONS_2026-06-20.md");

    for required in [
        "## Release-Flight Blockers",
        "UNBLOCK_V1_RELEASE",
        "## External Deferrals",
        "`cargo-deny` is still not installed",
        "RUSTSEC-2020-0168",
        "RUSTSEC-2024-0436",
        "RUSTSEC-2023-0071",
    ] {
        assert!(handoff.contains(required), "missing handoff marker {required:?}");
    }

    for forbidden in [
        "## Useful Follow-Up Enhancements",
        "Expand performance baselines",
        "Continue moving user-facing examples",
        "Keep root-surface hygiene strict",
    ] {
        assert!(
            !handoff.contains(forbidden),
            "next-session doc should list only blockers or external deferrals, found {forbidden:?}"
        );
    }
}

#[test]
fn dependency_refresh_evidence_note_records_removed_and_remaining_boundaries() {
    let note = read("notes/2026-06-20_v1_0_dependency_advisory_refresh.md");

    for marker in [
        "removed previously reported `core2`",
        "`proc-macro-error2` audit warnings",
        "`cargo audit --ignore RUSTSEC-2023-0071`: passed with warnings only",
        "No remaining dependency advisory item is locally actionable",
        "cargo-deny",
    ] {
        assert!(note.contains(marker), "missing dependency refresh note marker {marker:?}");
    }
}
