use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {dir:?}: {err}"));
    for entry in entries {
        let entry = entry.expect("failed to read directory entry");
        let path = entry.path();
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

        if path.is_dir() {
            if path.ends_with("docs/generated")
                || path.ends_with("examples/ssg/content")
                || file_name == "target"
            {
                continue;
            }
            collect_markdown_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
}

#[test]
fn unchecked_markdown_rows_are_only_release_flight_blockers_or_archived_notes() {
    let root = repo_root();
    let mut files = Vec::new();
    for top_level in ["README.md", "ROADMAP.md"] {
        files.push(root.join(top_level));
    }
    for top_level in ["docs", "examples"] {
        collect_markdown_files(&root.join(top_level), &mut files);
    }
    files.sort();
    files.dedup();

    let allowed_release_flight_files = [
        "docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md",
        "docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md",
        "docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md",
    ];

    let mut unexpected = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read markdown file {path:?}: {err}"));
        let relative = path
            .strip_prefix(&root)
            .expect("markdown path should be under repo root")
            .to_string_lossy()
            .replace('\\', "/");
        let allowed = allowed_release_flight_files.contains(&relative.as_str());

        for (line_index, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("- [ ]") && !allowed {
                unexpected.push(format!("{relative}:{}:{line}", line_index + 1));
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "unchecked markdown checklist rows outside release-flight docs should be triaged or converted to non-checkbox guidance:\n{}",
        unexpected.join("\n")
    );
}
