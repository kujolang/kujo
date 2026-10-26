//! Task callbacks retain local writes without publishing their isolated globals.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn task_callbacks_preserve_global_isolation_and_captured_locals() {
    let module_source = r###"mut global_value := 7
export func factory(relay) {
 mut count := 0
 func step(write) {
  if write {
   global_value = 9
   count += 1
   return relay(step)
  }
  return [global_value,count]
 }
 return step
}
export func local_relay(step) { return step(false) }
export func factory_sort(relay) {
 mut count := 0
 func step(write) {
  if write {
   global_value = 9
   count += 1
   sort_by([step,step],relay)
  }
  return [global_value,count]
 }
 return step
}
"###;
    let cases = [
        (
            "caller_relay",
            r###"from module_state import factory
func relay(step) { return step(false) }
let step := factory(relay)
print(step(false))
print(await await_task(spawn_task(func() { return step(true) })))
print(step(false))
"###,
            "[7, 0]\n[9, 1]\n[7, 1]\nPASS",
        ),
        (
            "module_relay",
            r###"from module_state import factory, local_relay
let step := factory(local_relay)
print(step(false))
print(await await_task(spawn_task(func() { return step(true) })))
print(step(false))
"###,
            "[7, 0]\n[9, 1]\n[7, 1]\nPASS",
        ),
        (
            "reused_sort_relay",
            r###"from module_state import factory_sort
func relay(left,right) { print(left(false)) return 0 }
let step := factory_sort(relay)
print(step(false))
print(await await_task(spawn_task(func() { return step(true) })))
print(step(false))
"###,
            "[7, 0]\n[9, 1]\n[9, 1]\n[7, 1]\nPASS",
        ),
    ];
    let mut failures = Vec::new();
    for (name, source, expected) in cases {
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_task_callback_globals_{}_{}_{}",
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
