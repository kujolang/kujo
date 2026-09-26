use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("kujo_vm_runtime_mismatch_inventory_{prefix}_{nanos}"));
    fs::create_dir_all(&path).expect("failed to create temp directory");
    path
}

fn script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("generate_vm_runtime_mismatch_inventory.sh")
}

#[test]
fn vm_runtime_mismatch_inventory_script_generates_expected_outputs() {
    let dir = unique_temp_dir("success");
    let output_md = dir.join("inventory.md");
    let output_csv = dir.join("inventory.csv");

    let output = Command::new("bash")
        .arg(script_path())
        .arg("--tests-dir")
        .arg("tests")
        .arg("--output-md")
        .arg(&output_md)
        .arg("--output-csv")
        .arg(&output_csv)
        .arg("--runner")
        .arg("target/debug/kujo")
        .arg("--max-fixtures")
        .arg("8")
        .arg("--strict")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to execute inventory script");

    assert!(
        output.status.success(),
        "inventory script should succeed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = fs::read_to_string(&output_md).expect("inventory markdown should exist");
    assert!(markdown.contains("# VM Runtime Mismatch Inventory"));
    assert!(markdown.contains("| Fixture | VM Exit | Interpreter Exit | VM Matches Snapshot | Interpreter Matches Snapshot | Delta Type | Mismatch Bucket | Owner | Priority | Rationale |"));
    assert!(markdown.contains("Summary: `8` fixtures scanned"));
    assert!(markdown.contains("Mismatch classification totals (priority order):"));
    assert!(markdown.contains("runtime-parity-bug"));
    assert!(markdown.contains("VM coverage gate:"));
    assert!(markdown.contains("metric: `vm_matches_snapshot / fixtures_scanned`"));
    assert!(markdown.contains("target threshold: `70.0%`"));
    assert!(markdown.contains("gate status: `"));

    let csv = fs::read_to_string(&output_csv).expect("inventory csv should exist");
    assert!(csv.contains("fixture,vm_exit,interpreter_exit,vm_matches_snapshot,interpreter_matches_snapshot,delta_type,mismatch_bucket,bucket_owner,priority,rationale"));
    assert!(
        !csv.contains("tests/fixtures/"),
        "inventory should mirror kujo test and exclude nested support fixtures"
    );
    assert!(
        !csv.contains("jit_direct_recursion.kujo"),
        "inventory should mirror kujo test and exclude test-run-only fixtures"
    );
    assert!(
        !csv.contains("postgres_tls_probe.kujo"),
        "inventory should exclude fixtures that require live external services"
    );

    let mut mismatch_rows = 0usize;
    let mut has_classified_mismatch = false;
    for line in csv.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 10 {
            continue;
        }
        let delta_type = parts[5];
        let mismatch_bucket = parts[6];
        if delta_type != "both_match_snapshot" {
            mismatch_rows += 1;
            if mismatch_bucket != "none" {
                has_classified_mismatch = true;
            }
        }
    }

    assert!(
        mismatch_rows == 0 || has_classified_mismatch,
        "expected mismatch rows to be classified when present"
    );
}

#[test]
fn vm_runtime_mismatch_inventory_script_is_deterministic_for_capped_scan() {
    let dir = unique_temp_dir("determinism");
    let output_md_a = dir.join("inventory_a.md");
    let output_csv_a = dir.join("inventory_a.csv");
    let output_md_b = dir.join("inventory_b.md");
    let output_csv_b = dir.join("inventory_b.csv");

    let run_once = |output_md: &PathBuf, output_csv: &PathBuf| {
        let output = Command::new("bash")
            .arg(script_path())
            .arg("--tests-dir")
            .arg("tests")
            .arg("--output-md")
            .arg(output_md)
            .arg("--output-csv")
            .arg(output_csv)
            .arg("--runner")
            .arg("target/debug/kujo")
            .arg("--max-fixtures")
            .arg("4")
            .arg("--strict")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("failed to execute inventory script");

        assert!(
            output.status.success(),
            "inventory script should succeed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run_once(&output_md_a, &output_csv_a);
    run_once(&output_md_b, &output_csv_b);

    let markdown_a = fs::read_to_string(output_md_a).expect("first markdown output should exist");
    let markdown_b = fs::read_to_string(output_md_b).expect("second markdown output should exist");
    assert_eq!(markdown_a, markdown_b, "markdown output should be deterministic");

    let csv_a = fs::read_to_string(output_csv_a).expect("first csv output should exist");
    let csv_b = fs::read_to_string(output_csv_b).expect("second csv output should exist");
    assert_eq!(csv_a, csv_b, "csv output should be deterministic");
}

#[test]
fn interpreter_override_mismatch_is_a_parity_bug() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let relative = format!("target/inventory_override_{nonce}");
    let dir = root.join(&relative);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("sample.kujo"), "print(\"actual\")\n").unwrap();
    fs::write(dir.join("sample.out"), "actual\n").unwrap();
    fs::write(dir.join("sample.interpreter.out"), "wrong\n").unwrap();
    let output = Command::new("bash")
        .arg(script_path())
        .args(["--tests-dir", &relative, "--runner", "target/debug/kujo", "--strict"])
        .arg("--output-csv")
        .arg(dir.join("inventory.csv"))
        .arg("--output-md")
        .arg(dir.join("inventory.md"))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let csv = fs::read_to_string(dir.join("inventory.csv")).unwrap();
    assert!(
        csv.contains(",yes,no,interpreter_only_mismatch,runtime-parity-bug,runtime-owner,P0,"),
        "{csv}"
    );
    fs::remove_dir_all(dir).unwrap();
}
