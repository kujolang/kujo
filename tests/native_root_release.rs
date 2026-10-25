use std::process::Command;

#[test]
fn native_root_discard_preserves_snapshots_returns_and_error_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("release.kujo");
    std::fs::write(
        &script,
        r#"
        func fail() { throw("expected root unwind") }
        func work(n, input) {
            mut local := input
            local[0] = n
            local[1][0] = n + 1
            let temporary := [local, str(n), {"number": n}]
            if n < 0 { fail() }
            return local[0] + len(temporary)
        }
        let snapshot := [7,[11,12]]
        mut i := 0
        while i < 300 {
            assert_equal(work(i,snapshot),i+3)
            if i % 17 == 0 {
                mut caught := false
                try { work(-1,snapshot) }
                except error { caught = contains(error.message,"expected root unwind") == 1 }
                assert(caught,"error must propagate after native root cleanup")
            }
            assert_equal(snapshot,[7,[11,12]])
            i += 1
        }
        assert_equal(work(42,snapshot),45)
        print("PASS")
    "#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let metrics = directory.path().join(format!("{mode}.json"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        if mode != "interpreter" {
            command.arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&script).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode}");
        if mode != "interpreter" {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            let entries = report["counters"]["jit_native_function_entries"].as_u64().unwrap();
            if mode == "jit" {
                assert!(entries > 0, "native body was not entered: {report}");
            } else {
                assert_eq!(entries, 0);
            }
        }
    }
}
