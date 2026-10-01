use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kujo-run"))
        .args(args)
        .output()
        .expect("failed to execute kujo-run")
}

fn expected_output(path: &Path) -> String {
    fs::read_to_string(path).unwrap().trim_end().to_string()
}

#[test]
fn lean_runner_matches_runtime_matrix_outputs_with_and_without_jit() {
    let suite = repo_root().join("benchmarks/cross-language/kujo-go-runtime");
    for workload in ["startup", "prime_count", "integer_mix"] {
        let script = suite.join(format!("{workload}.kujo"));
        let expected = expected_output(&suite.join("expected").join(format!("{workload}.txt")));
        for prefix in [Vec::<&str>::new(), vec!["--jit"]] {
            let mut args = prefix;
            args.push(script.to_str().unwrap());
            let output = run(&args);
            assert!(output.status.success(), "{workload}: {output:?}");
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim_end(), expected);
        }
    }
}

#[test]
fn lean_runner_help_version_and_usage_are_stable() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout).unwrap().contains("Usage: kujo-run [--jit] FILE"));

    let version = run(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        format!("kujo-run {}", env!("CARGO_PKG_VERSION"))
    );

    let invalid = run(&[]);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8(invalid.stderr).unwrap().contains("Usage: kujo-run"));
}
