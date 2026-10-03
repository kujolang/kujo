use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(path: &Path, interpreter: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.current_dir(repository_root()).arg("run");
    if interpreter {
        command.arg("--interpreter");
    }
    command.arg(path);
    command.output().expect("Kujo workload should start")
}

#[test]
fn representative_v1_8_workloads_preserve_vm_interpreter_parity() {
    let workloads = [
        ("data_cli.kujo", "{\"count\":3,\"total\":558,\"values\":[24,102,432]}\n"),
        ("generator_heavy.kujo", "80720000\n"),
        ("async_concurrent.kujo", "250500\n"),
        ("mixed_application.kujo", "cpu:66\n"),
        ("project/main.kujo", "160\n"),
    ];

    for (relative, expected) in workloads {
        let path = repository_root().join("benchmarks/v1-8-longitudinal").join(relative);
        let vm = run(&path, false);
        let interpreter = run(&path, true);

        assert!(
            vm.status.success(),
            "VM workload {relative} failed: {}",
            String::from_utf8_lossy(&vm.stderr)
        );
        assert!(
            interpreter.status.success(),
            "interpreter workload {relative} failed: {}",
            String::from_utf8_lossy(&interpreter.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&vm.stdout), expected, "VM output: {relative}");
        assert_eq!(
            String::from_utf8_lossy(&interpreter.stdout),
            expected,
            "interpreter output: {relative}"
        );
    }
}
