use std::process::Command;

#[test]
fn synchronous_parallel_workers_inherit_explicit_jit_opt_in() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("worker.kujo");
    std::fs::write(
        &script,
        include_str!("../benchmarks/langarena/probes/worker_jit_coverage.kujo"),
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let metrics = directory.path().join(format!("{mode}.json"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted"]);
        if mode != "interpreter" {
            command.arg("--measurements").arg(&metrics);
        }
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&script).arg("parallel").output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "worker coverage output verified"
        );
        if mode == "interpreter" {
            continue;
        }
        let report: serde_json::Value =
            serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
        let entries = report["counters"]["jit_native_function_entries"].as_u64().unwrap();
        if mode == "jit" {
            // The script never invokes the kernel in its parent VM. These
            // entries must therefore come from the two synchronous workers.
            assert!(entries >= 2, "workers did not enter native code: {report}");
        } else {
            assert_eq!(entries, 0, "JIT was enabled without opting in: {mode}");
        }
    }
}

#[test]
fn parallel_worker_policy_preserves_snapshots_isolation_order_and_errors() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("semantics.kujo");
    std::fs::write(
        &script,
        r#"
        mut marker := 7
        func work(input) {
            mut values := input
            values[0] += 1
            return [values, marker]
        }
        let original := [10, 20]
        let result := await parallel_map([original, [30, 40], original], work, 2)
        assert_equal(result, [[[11, 20], 7], [[31, 40], 7], [[11, 20], 7]])
        assert_equal(original, [10, 20])
        assert_equal(marker, 7)
        func denied(n) { read_file("must-not-open") return n }
        mut caught := false
        try { await parallel_map([1], denied, 1) }
        except error { caught = contains(error.message, "Capability denied") == 1 }
        assert(caught, "worker must inherit capability denial")
        func fail(n) { throw("mapper failed") }
        caught = false
        try { await parallel_map([1], fail, 1) }
        except error { caught = contains(error.message, "mapper failed") == 1 }
        assert(caught, "mapper failure must reach caller")
        assert_equal(await parallel_map([original], work, 1), [[[11, 20], 7]])
        print("PASS")
    "#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&script).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}
