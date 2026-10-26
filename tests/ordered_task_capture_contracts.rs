//! Channel ordering establishes visibility without assuming atomic read-modify-write.
//! Keep the interpreter gate explicit until shared active capture storage is fixed.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn verify(runtime: &str) {
    let mut failures = Vec::new();
    for (kind, initial, access) in [
        ("scalar", "1", ""),
        ("array", "[1]", "[0]"),
        ("dictionary", "{\"value\": 1}", "[\"value\"]"),
        ("nested_row", "[[1]]", "[0][0]"),
    ] {
        for worker_writes in [true, false] {
            let (worker, caller) = if worker_writes {
                (
                    format!("data{access} = 9 ready.send(1) proceed.receive() return 0"),
                    format!("ready.receive() let observed := data{access} proceed.send(1) await await_task(handle) return observed"),
                )
            } else {
                (
                    format!("ready.send(1) proceed.receive() return data{access}"),
                    format!("ready.receive() data{access} = 9 proceed.send(1) return await await_task(handle)"),
                )
            };
            let source = format!(
                r#"
let ready := channel()
let proceed := channel()
func factory() {{
    mut data := {initial}
    let saved := data
    func step(n) {{
        if n == 0 {{ {worker} }}
        if n == 2 {{ return data{access} }}
        if n == 3 {{ return saved{access} }}
        let handle := spawn_task(func() {{ return step(0) }})
        {caller}
    }}
    return step
}}
let step := factory()
print(step(1))
print(step(2))
print(step(3))
"#
            );
            let directory = std::env::temp_dir().join(format!(
                "kujo_ordered_capture_{}_{}_{}_{}",
                std::process::id(),
                runtime,
                kind,
                worker_writes
            ));
            fs::create_dir_all(&directory).unwrap();
            let script = directory.join("fixture.kujo");
            let stdout = directory.join("stdout");
            let stderr = directory.join("stderr");
            fs::write(&script, source).unwrap();
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
            // This is a deadlock bound, not a performance assertion. Channel
            // handshakes, rather than sleep durations, establish the ordering.
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
            if !status.is_some_and(|status| status.success()) || output != "9\n9\n1\n" {
                failures.push(format!(
                    "{runtime}/{kind}/worker_writes={worker_writes}: {status:?}, stdout={output:?}, stderr={error:?}"
                ));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn vm_channel_ordered_capture_visibility_preserves_snapshots() {
    verify("vm");
}

#[test]
fn jit_channel_ordered_capture_visibility_preserves_snapshots() {
    verify("jit");
}

#[test]
#[ignore = "Known interpreter active-capture visibility defect; run explicitly as the pending fix gate"]
fn interpreter_channel_ordered_capture_visibility_preserves_snapshots() {
    verify("interpreter");
}
