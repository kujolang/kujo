//! Indexed arithmetic must preserve operator callbacks, snapshots and errors.
use std::process::Command;

#[test]
fn compound_indexed_writes_preserve_publication_and_selector_order() {
    for (path, expected) in [
        ("tests/fixtures/compound_publication/success/main.kujo", "COMPOUND CALLBACK PASS\n"),
        ("tests/fixtures/compound_publication/error/main.kujo", "COMPOUND CALLBACK PASS\n"),
        (
            "tests/fixtures/compound_publication/selector-replacement/main.kujo",
            "SELECTOR SNAPSHOT PASS\n",
        ),
        (
            "tests/fixtures/compound_publication/callback-selector/main.kujo",
            "SELECTOR SNAPSHOT PASS\n",
        ),
        (
            "benchmarks/langarena/probes/compound-type-transition/main.kujo",
            "COMPOUND TYPE TRANSITION PASS\n",
        ),
    ] {
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command
                .arg("run")
                .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
                .args(["--untrusted", "--scheduler-no-timeout"]);
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{path}, {mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, expected.as_bytes(), "{path}, {mode}");
        }
    }
}
