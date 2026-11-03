use std::process::Command;

fn run_source(source: &str, mode: &str) -> std::process::Output {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "kujo-call-contract-{}-{}.kujo",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--scheduler-no-timeout"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    let output = command.arg(&path).output();
    std::fs::remove_file(path).unwrap();
    output.unwrap()
}

#[test]
fn cold_invalid_calls_unwind_to_caller_and_recover() {
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        for invalid in ["target()", "target(1, 2)", "number(1)"] {
            let source = format!(
                r#"
                func target(x) {{ return x }}
                let number := 17
                func relay() {{ {invalid} return 999 }}
                let snapshot := [[1, 2]]
                mut recovered := 0
                try {{ relay() }} except error {{ recovered += 1 }}
                assert_equal(recovered, 1)
                assert_equal(target(7), 7)
                assert_equal(snapshot, [[1, 2]])
                print("recovered")
                "#
            );
            let output = run_source(&source, mode);
            assert!(
                output.status.success(),
                "{mode} {invalid}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "recovered");
        }
    }
}

#[test]
fn uncaught_invalid_calls_keep_runtime_failure_exit_code() {
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        for (invalid, diagnostic) in
            [("target()", "expects"), ("target(1, 2)", "expects"), ("number(1)", "non-function")]
        {
            let source = format!("func target(x) {{ return x }} let number := 17 {invalid}");
            let output = run_source(&source, mode);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(4), "{mode} {invalid}: {stderr}");
            assert!(stderr.contains(diagnostic), "{mode} {invalid}: {stderr}");
        }
    }
}

#[test]
fn hot_calls_preserve_rebinding_argument_order_snapshots_and_recovery() {
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/benchmarks/langarena/probes/native-call-contracts.kujo"
            ))
            .output()
            .unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "native-call-contracts: OK");
    }
}
