use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn vm_runtime_mismatch_markdown_artifact_contains_required_markers() {
    let root = repo_root();
    let markdown_path =
        root.join("docs").join("generated").join("VM_RUNTIME_MISMATCH_INVENTORY.md");
    let markdown =
        fs::read_to_string(markdown_path).expect("generated markdown artifact should exist");

    assert!(markdown.contains("# VM Runtime Mismatch Inventory"));
    assert!(markdown.contains("| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |"));
    assert!(markdown.contains("Summary: `"));
    assert!(markdown.contains("Mismatch classification totals (priority order):"));
    assert!(markdown.contains("runtime-parity-bug"));
    assert!(markdown.contains("stale-snapshot-expectation"));
    assert!(markdown.contains("harness-debt"));
}

#[test]
fn vm_runtime_mismatch_csv_artifact_classifies_every_mismatch_row() {
    let root = repo_root();
    let csv_path = root.join("docs").join("generated").join("VM_RUNTIME_MISMATCH_INVENTORY.csv");
    let csv = fs::read_to_string(csv_path).expect("generated csv artifact should exist");

    let mut lines = csv.lines();
    let header = lines.next().expect("csv should include header");
    assert_eq!(
        header,
        "fixture,vm_exit,interpreter_exit,vm_matches_snapshot,interpreter_matches_snapshot,delta_type,mismatch_bucket,bucket_owner,priority,rationale"
    );

    let valid_buckets: HashSet<&str> = HashSet::from([
        "none",
        "parser-invalid-fixture",
        "stale-snapshot-expectation",
        "runtime-parity-bug",
        "intentional-divergence",
        "harness-debt",
    ]);
    let valid_priorities: HashSet<&str> = HashSet::from(["P0", "P1", "P2", "P4"]);

    let mut mismatch_rows = 0usize;
    let mut total_rows = 0usize;
    let mut all_rows_are_baseline_matches = true;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        total_rows += 1;
        let parts: Vec<&str> = line.split(',').collect();
        assert!(parts.len() >= 10, "csv row should contain 10 columns: {line}");

        let delta_type = parts[5];
        let bucket = parts[6];
        let owner = parts[7];
        let priority = parts[8];

        assert!(valid_buckets.contains(bucket), "unexpected bucket {bucket} in row: {line}");
        assert!(
            valid_priorities.contains(priority),
            "unexpected priority {priority} in row: {line}"
        );

        if delta_type != "both_match_snapshot" {
            all_rows_are_baseline_matches = false;
            mismatch_rows += 1;
            assert_ne!(bucket, "none", "mismatch row must not have bucket=none: {line}");
            assert_ne!(owner, "n/a", "mismatch row must not have owner=n/a: {line}");
            assert_ne!(priority, "P4", "mismatch row must not have priority P4: {line}");
        }
    }

    assert!(total_rows > 0, "expected at least one baseline row in inventory artifact");
    if mismatch_rows == 0 {
        assert!(
            all_rows_are_baseline_matches,
            "when mismatch count is zero, every row should be both_match_snapshot"
        );
    }
}

#[test]
fn vm_runtime_mismatch_generator_strict_mode_succeeds_for_repo_scan() {
    let root = repo_root();
    let temp_dir = std::env::temp_dir().join("kujo_vm_runtime_mismatch_strict_contract");
    fs::create_dir_all(&temp_dir).expect("temp dir should be creatable");
    let output_md = temp_dir.join("inventory.md");
    let output_csv = temp_dir.join("inventory.csv");
    let output = Command::new("bash")
        .current_dir(&root)
        .args([
            "scripts/generate_vm_runtime_mismatch_inventory.sh",
            "--runner",
            "target/debug/kujo",
            "--max-fixtures",
            "6",
            "--output-md",
            output_md.to_str().expect("temp output markdown path should be utf-8"),
            "--output-csv",
            output_csv.to_str().expect("temp output csv path should be utf-8"),
            "--strict",
        ])
        .output()
        .expect("failed to run strict inventory generation");

    assert!(
        output.status.success(),
        "strict mode should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn vm_runtime_mismatch_baseline_does_not_bucket_runtime_divergence_as_harness_debt() {
    let root = repo_root();
    let csv_path = root.join("docs").join("generated").join("VM_RUNTIME_MISMATCH_INVENTORY.csv");
    let csv = fs::read_to_string(csv_path).expect("generated csv artifact should exist");

    for line in csv.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        assert!(parts.len() >= 10, "csv row should contain 10 columns: {line}");
        let delta_type = parts[5];
        let bucket = parts[6];
        if delta_type == "both_mismatch_different_output" {
            assert_ne!(
                bucket, "harness-debt",
                "runtime divergence rows should not be classified as harness debt: {line}"
            );
        }
    }
}
