//! Unary operator callbacks publish preceding indexed writes before entering user code.
use std::process::Command;

#[test]
fn indexed_unary_operator_preserves_prior_writes_and_snapshots() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/compound_publication/unary/main.kujo");
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&path).args(["--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"COMPOUND UNARY PASS\n", "{mode}");
    }
}
