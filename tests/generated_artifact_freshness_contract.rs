use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FRESHNESS_MAX_AGE_DAYS: i64 = 7;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn normalized_markdown(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for line in text.lines() {
        let rewritten = if line.starts_with("Generated: ") {
            "Generated: <normalized>".to_string()
        } else if line.starts_with("Date: ") {
            "Date: <normalized>".to_string()
        } else if line.starts_with("Runner: ") {
            "Runner: <normalized>".to_string()
        } else {
            line.to_string()
        };
        normalized.push_str(&rewritten);
        normalized.push('\n');
    }
    normalized
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {}", path.display(), error))
}

fn run_script(script: &str, args: &[&str], output_md: &Path, output_csv: &Path) {
    let output = run_script_with_env(script, args, &[], output_md, output_csv);

    assert!(
        output.status.success(),
        "{} should succeed: stdout={} stderr={}",
        script,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output_md.exists(), "{script} should write markdown output");
    assert!(output_csv.exists(), "{script} should write csv output");
}

fn run_script_with_env(
    script: &str,
    args: &[&str],
    envs: &[(&str, &str)],
    output_md: &Path,
    output_csv: &Path,
) -> std::process::Output {
    let mut command = Command::new("bash");
    command.arg(repo_root().join("scripts").join(script)).args(args).current_dir(repo_root());
    for (name, value) in envs {
        command.env(name, value);
    }

    let output =
        command.output().unwrap_or_else(|error| panic!("failed to execute {script}: {error}"));

    assert!(
        output.status.success(),
        "{} should succeed: stdout={} stderr={}",
        script,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(output_md.exists(), "{script} should write markdown output");
    assert!(output_csv.exists(), "{script} should write csv output");

    output
}

fn assert_fresh_date(path: &Path, prefix: &str) {
    let content = read_text(path);
    let date_line = content
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("{} should contain a {} line", path.display(), prefix));
    let raw_date = date_line
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("{} should parse with {} prefix", path.display(), prefix))
        .trim();
    let generated_date = NaiveDate::parse_from_str(raw_date, "%Y-%m-%d")
        .expect("generated date should be YYYY-MM-DD");
    let today = chrono::Utc::now().date_naive();
    let age_days = today.signed_duration_since(generated_date).num_days();
    assert!(
        (0..=FRESHNESS_MAX_AGE_DAYS).contains(&age_days),
        "{} should be regenerated within {} days (found {} days old, date={})",
        path.display(),
        FRESHNESS_MAX_AGE_DAYS,
        age_days,
        raw_date
    );
}

fn assert_normalized_file_match(script_output: &Path, checked_in: &Path) {
    let generated = normalized_markdown(&read_text(script_output));
    let committed = normalized_markdown(&read_text(checked_in));
    assert_eq!(
        committed,
        generated,
        "generated artifact drifted from checked-in baseline: {}",
        checked_in.display()
    );
}

#[test]
fn generated_todo_triage_artifact_is_fresh_and_matches_generator_output() {
    let temp_dir = std::env::temp_dir()
        .join(format!("kujo_generated_artifact_freshness_todo_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let output_md = temp_dir.join("todo.md");
    let output_csv = temp_dir.join("todo.csv");

    run_script(
        "generate_v1_code_todo_triage.sh",
        &[
            "--strict",
            "--output-md",
            output_md.to_str().expect("utf-8"),
            "--output-csv",
            output_csv.to_str().expect("utf-8"),
        ],
        &output_md,
        &output_csv,
    );

    let checked_in_md = repo_root().join("docs/generated/V1_CODE_TODO_TRIAGE.md");
    let checked_in_csv = repo_root().join("docs/generated/V1_CODE_TODO_TRIAGE.csv");

    assert_fresh_date(&checked_in_md, "Generated: ");
    assert_fresh_date(&output_md, "Generated: ");
    assert_normalized_file_match(&output_md, &checked_in_md);
    assert_eq!(read_text(&checked_in_csv), read_text(&output_csv));
}

#[test]
fn generated_unsafe_inventory_artifact_is_fresh_and_matches_generator_output() {
    let temp_dir = std::env::temp_dir()
        .join(format!("kujo_generated_artifact_freshness_unsafe_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let output_md = temp_dir.join("unsafe.md");
    let output_csv = temp_dir.join("unsafe.csv");

    run_script(
        "generate_unsafe_inventory.sh",
        &[
            "--strict",
            "--output-md",
            output_md.to_str().expect("utf-8"),
            "--output-csv",
            output_csv.to_str().expect("utf-8"),
        ],
        &output_md,
        &output_csv,
    );

    let checked_in_md = repo_root().join("docs/generated/UNSAFE_INVENTORY.md");
    let checked_in_csv = repo_root().join("docs/generated/UNSAFE_INVENTORY.csv");

    assert_fresh_date(&checked_in_md, "Generated: ");
    assert_fresh_date(&output_md, "Generated: ");
    assert_normalized_file_match(&output_md, &checked_in_md);
    assert_eq!(read_text(&checked_in_csv), read_text(&output_csv));
}

#[test]
fn generated_vm_mismatch_inventory_artifact_is_fresh_and_matches_generator_output() {
    let temp_dir = std::env::temp_dir()
        .join(format!("kujo_generated_artifact_freshness_vm_mismatch_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");
    let output_md = temp_dir.join("vm_mismatch.md");
    let output_csv = temp_dir.join("vm_mismatch.csv");
    let runner = repo_root().join("target").join("debug").join("kujo");

    run_script_with_env(
        "generate_vm_runtime_mismatch_inventory.sh",
        &[
            "--strict",
            "--runner",
            runner.to_str().expect("runner should be utf-8"),
            "--output-md",
            output_md.to_str().expect("utf-8"),
            "--output-csv",
            output_csv.to_str().expect("utf-8"),
        ],
        &[
            ("KUJO_MISMATCH_FIXTURE_TIMEOUT_SECONDS", "60"),
            ("KUJO_MISMATCH_CRYPTO_FIXTURE_TIMEOUT_SECONDS", "300"),
        ],
        &output_md,
        &output_csv,
    );

    let checked_in_md = repo_root().join("docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md");
    let checked_in_csv = repo_root().join("docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.csv");

    assert_fresh_date(&checked_in_md, "Generated: ");
    assert_fresh_date(&output_md, "Generated: ");
    let generated_md = normalized_markdown(&read_text(&output_md));
    let committed_md = normalized_markdown(&read_text(&checked_in_md));
    for needle in [
        "# VM Runtime Mismatch Inventory",
        "| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |",
        "Summary:",
        "VM coverage gate:",
        "gate status: `PASS`",
    ] {
        assert!(
            generated_md.contains(needle),
            "generated mismatch inventory should contain {needle:?}"
        );
        assert!(
            committed_md.contains(needle),
            "checked-in mismatch inventory should contain {needle:?}"
        );
    }
    let generated_csv = read_text(&output_csv);
    let committed_csv = read_text(&checked_in_csv);
    let expected_csv_header = "fixture,vm_exit,interpreter_exit,vm_matches_snapshot,interpreter_matches_snapshot,delta_type,mismatch_bucket,bucket_owner,priority,rationale";
    assert!(generated_csv.lines().next() == Some(expected_csv_header));
    assert!(committed_csv.lines().next() == Some(expected_csv_header));
    assert!(
        generated_csv.lines().count() > 1,
        "generated mismatch inventory should include data rows"
    );
    assert!(
        committed_csv.lines().count() > 1,
        "checked-in mismatch inventory should include data rows"
    );
}
