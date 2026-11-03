use std::process::Command;

#[test]
fn scalar_indexed_writes_preserve_snapshots_types_and_errors() {
    let source = include_str!("fixtures/jit_scalar_indexed_write.kujo");
    let path = std::env::temp_dir().join(format!("kujo-scalar-write-{}.kujo", std::process::id()));
    std::fs::write(&path, source).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "scalar-indexed-write: OK");
    }
    std::fs::remove_file(path).unwrap();
}
