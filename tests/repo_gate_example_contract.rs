use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn examples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kujo"))
        .current_dir(examples_root())
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("repo_gate example should execute")
}

fn expected(name: &str) -> Value {
    let path = examples_root().join("repo_gate/expected").join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("expected report should be readable"))
        .expect("expected report should be valid JSON")
}

#[test]
fn helper_smoke_runs_without_module_path_wiring() {
    let output = run(&["run", "repo_gate/repo_gate_test.kujo"]);
    assert!(
        output.status.success(),
        "helper smoke failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("repo_gate helper smoke passed"));
}

#[test]
fn passing_and_failing_reports_are_deterministic() {
    for (fixture, report, expected_code) in
        [("passing_project", "pass-report.json", 0), ("failing_project", "fail-report.json", 1)]
    {
        let output = run(&[
            "run",
            "repo_gate/repo_gate.kujo",
            "--",
            "--root",
            &format!("repo_gate/fixtures/{fixture}"),
            "--policy",
            "repo_gate/gate-policy.json",
            "--stdout-json",
        ]);
        assert_eq!(
            output.status.code(),
            Some(expected_code),
            "unexpected status for {fixture}: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "invalid JSON for {fixture}: {error}; stdout={}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
        assert_eq!(actual, expected(report), "report drift for {fixture}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Repo Gate Audit Complete"));
    }
}
