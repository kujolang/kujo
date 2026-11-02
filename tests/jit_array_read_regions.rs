use std::process::Command;

#[test]
fn borrowed_array_regions_preserve_snapshots_callbacks_and_fallbacks() {
    for (fixture, expected) in [
        ("native-region-contracts.kujo", "native-region-contracts: OK"),
        ("native-region-boundaries.kujo", "native-region-boundaries: OK"),
    ] {
        for mode in ["vm", "interpreter", "jit"] {
            if mode == "jit" && !cfg!(feature = "runtime-jit") {
                continue;
            }
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command
                .arg(format!(
                    "{}/benchmarks/langarena/probes/{fixture}",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{fixture} {mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).trim(),
                expected,
                "{fixture} {mode}"
            );
        }
    }
}
