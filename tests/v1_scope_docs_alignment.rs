use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn v1_scope_docs_keep_deferred_boundaries_aligned() {
    let root = repo_root();

    let readme =
        fs::read_to_string(root.join("README.md")).expect("failed to read README.md for alignment");
    let v1_scope = fs::read_to_string(root.join("docs").join("V1_SCOPE.md"))
        .expect("failed to read docs/V1_SCOPE.md for alignment");
    let optional_typing = fs::read_to_string(root.join("docs").join("OPTIONAL_TYPING_DESIGN.md"))
        .expect("failed to read docs/OPTIONAL_TYPING_DESIGN.md for alignment");

    assert!(
        readme.contains(
            "The project is currently at `1.0.0` in `Cargo.toml` and the stable release tag is `v1.0.0`."
        ) && readme.contains("Prebuilt Linux x64, macOS x64/arm64, and Windows x64 binaries"),
        "README must keep explicit stable-release and artifact wording"
    );
    assert!(
        readme.contains("docs/V1_SCOPE.md") && readme.contains("docs/OPTIONAL_TYPING_DESIGN.md"),
        "README must link deferred/non-goal boundary sources"
    );

    assert!(
        v1_scope.contains("## Deferred Post-1.0 Candidates (Non-Blocking)"),
        "V1 scope doc must keep explicit deferred post-1.0 section"
    );
    for marker in ["Generics", "FFI (foreign function interface)", "WASM target", "Macro system"] {
        assert!(v1_scope.contains(marker), "V1 scope doc missing deferred marker {marker:?}");
    }

    assert!(
        optional_typing.contains("- Deferred after v1:")
            && optional_typing.contains("runtime type enforcement")
            && optional_typing.contains("mandatory static type checking gates in `kujo run`"),
        "optional typing policy must keep runtime/enforcement deferrals explicit"
    );
    assert!(
        optional_typing.contains("Any future runtime checks must remain opt-in"),
        "optional typing policy must keep opt-in enforcement boundary explicit"
    );

    let optional_typing_lower = optional_typing.to_lowercase();
    let v1_scope_lower = v1_scope.to_lowercase();
    for marker in [
        "destructuring inference",
        "module existence checks",
        "struct field type lookup",
        "promise unwrap typing",
        "permissive callable fallback",
    ] {
        assert!(
            optional_typing_lower.contains(marker),
            "optional typing policy missing post-v1 checker marker {marker:?}"
        );
        assert!(
            v1_scope_lower.contains(marker),
            "V1 scope doc missing post-v1 checker marker {marker:?}"
        );
    }
}
