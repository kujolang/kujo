use std::process::Command;

#[test]
fn indexed_fields_preserve_scalars_owned_values_snapshots_and_errors() {
    let source = include_str!("fixtures/jit_indexed_field_fusion.kujo");
    let path = std::env::temp_dir().join(format!("kujo-indexed-field-{}.kujo", std::process::id()));
    std::fs::write(&path, source).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let metrics = directory.path().join("metrics.json");
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        if mode == "jit" {
            command.arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "indexed-field-transfer: OK");
        if mode == "jit" {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&metrics).unwrap()).unwrap();
            for name in ["jit_indexed_field_fast_reads", "jit_indexed_field_fallback_reads"] {
                assert!(
                    report["counters"][name].as_u64().unwrap() > 0,
                    "missing exercised path {name}: {report}"
                );
            }
        }
    }
    std::fs::remove_file(path).unwrap();
}
