//! Nested module tasks and foreign callbacks use their own submission globals.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn nested_module_tasks_preserve_distinct_global_snapshots() {
    let module_source = r###"mut global_value := 7
async func inner() { return global_value }
export async func outer(callback) {
 global_value = 9
 return [await inner(),callback()]
}
export async func invoke(callback) { return callback() }
export func call_current(work) { return work(false) }
export async func invoke_twice(callback) { return [callback(1),callback(0)] }
export async func invoke_caught(callback) {
 try { callback(1) } except err {}
 return callback(0)
}
"###;
    let cases = [
        (
            "nested_module",
            r###"from module_state import outer
mut global_value := 42
print(await outer(func() { return global_value }))
print(global_value)
"###,
            "[9, 42]\n42\nPASS",
        ),
        (
            "callback_submission",
            r###"from module_state import invoke
mut global_value := 42
func factory() { return func() { return global_value } }
let callback := factory()
global_value = 43
print(await invoke(callback))
print(global_value)
"###,
            "43\n43\nPASS",
        ),
        (
            "callback_global_writes",
            r###"from module_state import invoke_twice
mut global_value := 42
func factory() {
 mut count := 0
 return func(write) {
  if write { global_value = 10 count += 1 }
  return [global_value,count]
 }
}
let callback := factory()
global_value = 43
print(await invoke_twice(callback))
print(global_value)
"###,
            "[[10, 1], [10, 1]]\n43\nPASS",
        ),
        (
            "callback_caught_global_write",
            r###"from module_state import invoke_caught
mut global_value := 42
func factory() {
 mut count := 0
 return func(write) {
  if write { count += 1 global_value = 10 throw("expected") }
  return global_value
 }
}
let callback := factory()
global_value = 43
print(await invoke_caught(callback))
print(global_value)
"###,
            "10\n43\nPASS",
        ),
        (
            "active_foreign_reentry",
            r###"from module_state import call_current
mut global_value := 0
func factory() {
 mut count := 0
 func work(write) {
  if !write { return [global_value,count] }
  global_value = 9
  count += 1
  return call_current(work)
 }
 return work
}
let work := factory()
print(await await_task(spawn_task(func() { return work(true) })))
print(global_value)
"###,
            "[9, 1]\n0\nPASS",
        ),
    ];
    let mut failures = Vec::new();
    for (name, source, expected) in cases {
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_nested_module_task_globals_{}_{}_{}",
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
            if !status.is_some_and(|status| status.success()) || output.trim() != expected {
                failures.push(format!("{name}/{runtime}: {status:?}, {output:?}, {error}"));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
