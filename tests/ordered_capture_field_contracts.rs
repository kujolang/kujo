//! Ordered field writes must survive errors and generator suspension.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn verify(runtime: &str) {
    let mut failures = Vec::new();
    {
        let directory = std::env::temp_dir().join(format!(
            "kujo_ordered_fields_{}_{}",
            std::process::id(),
            runtime
        ));
        fs::create_dir_all(&directory).unwrap();
        let script = directory.join("fixture.kujo");
        let stdout = directory.join("stdout");
        let stderr = directory.join("stderr");
        fs::write(&script, include_str!("fixtures/ordered_capture_fields_generator.kujo")).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.arg("run").arg(&script);
        if runtime != "vm" {
            command.arg(format!("--{runtime}"));
        }
        let mut child = command
            .stdout(Stdio::from(File::create(&stdout).unwrap()))
            .stderr(Stdio::from(File::create(&stderr).unwrap()))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(120);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let output = fs::read_to_string(&stdout).unwrap();
        let error = fs::read_to_string(&stderr).unwrap();
        if !status.is_some_and(|s| s.success())
            || output != "[9, 1, true]\n9\n1\n[9, 1, true]\n9\n1\n"
        {
            failures.push(format!("{runtime}: {status:?}, stdout={output:?}, stderr={error:?}"));
        }
        fs::remove_dir_all(directory).unwrap();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn vm_ordered_fields_preserve_generator_and_immutable_snapshots() {
    verify("vm");
}

#[test]
fn jit_ordered_fields_preserve_generator_and_immutable_snapshots() {
    verify("jit");
}

#[test]
fn interpreter_ordered_fields_preserve_generator_and_immutable_snapshots() {
    verify("interpreter");
}
