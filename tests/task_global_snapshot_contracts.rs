//! Task globals refresh at submission while closure locals persist.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn task_closures_refresh_submission_globals() {
    let cases = [
        (
            "ordinary",
            r###"mut global_value := 0
func factory() {
 mut local_value := 0
 func work(write) {
  if write { global_value = 9 local_value += 1 }
  return [global_value, local_value]
 }
 return work
}
let work := factory()
print(await await_task(spawn_task(func() { return work(true) })))
global_value = 42
print(await await_task(spawn_task(func() { return work(false) })))
print(global_value)
"###,
            "[9, 1]\n[42, 1]\n42\nPASS",
        ),
        (
            "async",
            r###"mut global_value := 0
func factory() {
 mut local_value := 0
 async func work(write) {
  if write { global_value = 9 local_value += 1 }
  return [global_value, local_value]
 }
 return work
}
let work := factory()
print(await work(true))
global_value = 42
print(await work(false))
print(global_value)
"###,
            "[9, 1]\n[42, 1]\n42\nPASS",
        ),
        (
            "nested_submission",
            r###"mut global_value := 0
async func inner() { return global_value }
async func outer() {
 global_value = 9
 return await inner()
}
print(await outer())
print(global_value)
"###,
            "9\n0\nPASS",
        ),
        (
            "nested_ordinary_submission",
            r###"mut global_value := 0
async func outer() {
 global_value = 9
 return await await_task(spawn_task(func() { return global_value }))
}
print(await outer())
print(global_value)
"###,
            "9\n0\nPASS",
        ),
    ];
    let mut failures = Vec::new();
    for (name, source, expected) in cases {
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_submission_task_globals_{}_{}_{}",
                std::process::id(),
                name,
                runtime
            ));
            fs::create_dir_all(&directory).unwrap();
            let script = directory.join("fixture.kujo");
            let stdout = directory.join("stdout");
            let stderr = directory.join("stderr");
            fs::write(&script, format!("{source}\nprint(\"PASS\")\n")).unwrap();
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
            let deadline = Instant::now() + Duration::from_secs(30);
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
            if !status.is_some_and(|status| status.success()) || output.trim() != expected {
                failures.push(format!("{name}/{runtime}: {status:?}, {output:?}, {error}"));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
