use std::fs;
use std::process::Command;

fn invoke(directory: Option<&std::path::Path>, command: &str) -> std::process::Output {
    let mut process = Command::new("bash");
    process.current_dir(env!("CARGO_MANIFEST_DIR"));
    process.args([
        "-c",
        &format!("set -euo pipefail; source scripts/release_gate_output.sh; {command}"),
    ]);
    process.env_remove("KUJO_RELEASE_GATE_LOG_DIR");
    if let Some(directory) = directory {
        process.env("KUJO_RELEASE_GATE_LOG_DIR", directory);
    }
    process.output().unwrap()
}

#[test]
fn receipt_mode_preserves_all_output_and_exact_failure_status() {
    let directory = tempfile::tempdir().unwrap();
    let command = "run_cmd bash -c 'printf stdout; printf stderr >&2; exit 7'; echo unreachable";
    let legacy = invoke(None, command);
    let receipt = invoke(Some(directory.path()), command);
    assert_eq!(legacy.status.code(), Some(7));
    assert_eq!(receipt.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&legacy.stderr).contains("stderr"));
    assert!(receipt.stderr.is_empty());
    let output = String::from_utf8(receipt.stdout).unwrap();
    assert!(output.contains("exit=7 log="));
    assert!(!output.contains("\nunreachable"));
    let run = fs::read_dir(directory.path()).unwrap().next().unwrap().unwrap().path();
    assert_eq!(fs::read_to_string(run.join("1.log")).unwrap(), "stdoutstderr");
}

#[test]
fn receipt_mode_bounds_output_and_never_overwrites_previous_runs() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("logs with spaces");
    let command = "run_cmd bash -c 'printf \"%100000s\" x'; run_cmd printf done";
    let legacy = invoke(None, command);
    assert!(legacy.status.success());
    for _ in 0..2 {
        let receipt = invoke(Some(&path), command);
        assert!(receipt.status.success());
        assert!(receipt.stdout.len() < 1024);
        assert!(legacy.stdout.len() > 100000);
        let mut interpreter = kujo::interpreter::Interpreter::new();
        let text = kujo::interpreter::Value::Str(std::sync::Arc::new(
            String::from_utf8(receipt.stdout).unwrap(),
        ));
        let estimate = interpreter.call_native_function_impl("ai_count_tokens", &[text]);
        assert!(matches!(estimate, kujo::interpreter::Value::Int(count) if count <= 256));
    }
    let runs: Vec<_> = fs::read_dir(&path).unwrap().map(|entry| entry.unwrap().path()).collect();
    assert_eq!(runs.len(), 2);
    for run in runs {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&run).unwrap().permissions().mode() & 0o777, 0o700);
        }
        assert_eq!(fs::metadata(run.join("1.log")).unwrap().len(), 100000);
        assert_eq!(fs::read_to_string(run.join("2.log")).unwrap(), "done");
    }
}

#[test]
fn receipt_mode_fails_closed_when_log_directory_cannot_be_created() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file");
    fs::write(&file, "retain").unwrap();
    let output = invoke(Some(&file), "run_cmd printf should-not-run");
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("should-not-run"));
    assert_eq!(fs::read_to_string(file).unwrap(), "retain");
}
