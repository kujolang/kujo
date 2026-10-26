//! Imported task closures retain module globals despite caller name collisions.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn imported_task_closures_preserve_module_globals() {
    let module_source = r###"let global_value := 7
func helper() { return global_value }
export func make_work() {
 mut local_value := 0
 func work() { local_value += 1 return [helper(),local_value] }
 return work
}
export func make_async_work() {
 mut local_value := 0
 async func work() { local_value += 1 return [helper(),local_value] }
 return work
}
"###;
    let cases = [
        (
            "ordinary",
            r###"from module_state import make_work
mut global_value := 42
let work := make_work()
print(await await_task(spawn_task(work)))
global_value = 99
print(await await_task(spawn_task(work)))
print(global_value)
"###,
        ),
        (
            "async",
            r###"from module_state import make_async_work
mut global_value := 42
let work := make_async_work()
print(await work())
global_value = 99
print(await work())
print(global_value)
"###,
        ),
    ];
    let mut failures = Vec::new();
    for (name, source) in cases {
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_module_task_globals_{}_{}_{}",
                std::process::id(),
                name,
                runtime
            ));
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("module_state.kujo"), module_source).unwrap();
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
            if !status.is_some_and(|status| status.success())
                || output.trim() != "[7, 1]\n[7, 2]\n99\nPASS"
            {
                failures.push(format!("{name}/{runtime}: {status:?}, {output:?}, {error}"));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
