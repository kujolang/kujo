use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parse_cache_ratio(content: &str) -> u32 {
    let line = content
        .lines()
        .find(|line| line.contains("approximately `") && line.contains("faster"))
        .expect("cache lookup artifact should contain ratio interpretation");
    let ratio_text = line
        .split('`')
        .nth(3)
        .expect("cache ratio should appear in backticks")
        .trim_end_matches('x');
    ratio_text.parse::<u32>().expect("cache ratio should parse as an integer")
}

#[test]
fn import_heavy_cache_lookup_artifact_contains_required_markers() {
    let path = repo_root().join("docs").join("generated").join("VM_IMPORT_HEAVY_CACHE_LOOKUP.md");
    let content = fs::read_to_string(&path)
        .expect("failed to read docs/generated/VM_IMPORT_HEAVY_CACHE_LOOKUP.md");

    for marker in [
        "# VM Import-Heavy Nested Lookup Cache Validation",
        "module_resolution/import_heavy_nested_dotted_startup_cold_loader",
        "module_resolution/import_heavy_nested_dotted_cached_lookup_warm_loader",
        "## Cache Effect Interpretation",
        "approximately `141x` faster",
        "src/module.rs::load_module_reuses_cached_nested_dotted_module_without_duplicate_cache_entries",
    ] {
        assert!(
            content.contains(marker),
            "cache lookup artifact should contain marker {:?}",
            marker
        );
    }

    let ratio = parse_cache_ratio(&content);
    assert!(
        ratio >= 100,
        "cache lookup ratio should show a strong warm-path benefit, got {}x",
        ratio
    );
}
