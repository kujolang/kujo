//! Task write journals preserve recursion, callback state and collection snapshots.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn ordinary_task_journals_preserve_nested_calls_and_snapshots() {
    let cases = [
        (
            "recursion",
            r#"
func factory() {
    mut a := 0
    mut b := 0
    func step(n) {
        if n == 0 { b = 2 return 0 }
        if n == 1 { a = 1 step(0) }
        return [a,b]
    }
    return step
}
let step := factory()
let task := spawn_task(func() { return step(1) })
assert_equal(await await_task(task), [1,2])
assert_equal(step(2), [1,2])
"#,
        ),
        (
            "callback",
            r#"
func factory() {
    mut a := 0
    mut b := 0
    func step(n) {
        if n == 0 { b = 2 return 0 }
        if n == 1 { a = 1 let ignored := map([0],step) }
        return [a,b]
    }
    return step
}
let step := factory()
let task := spawn_task(func() { return step(1) })
assert_equal(await await_task(task), [1,2])
assert_equal(step(2), [1,2])
"#,
        ),
        (
            "collection_snapshot",
            r#"
func factory() {
    mut data := [1]
    func step(n) {
        if n == 0 { let previous := data data[0] = 9 return previous }
        return data
    }
    return step
}
let step := factory()
let task := spawn_task(func() { return step(0) })
assert_equal(await await_task(task), [1])
assert_equal(step(1), [9])
"#,
        ),
        (
            "independent_closures",
            r#"
func factory() {
    mut data := [1]
    func step(n) { if n == 0 { data[0] = 9 } return data }
    return step
}
let left := factory()
let right := factory()
let task := spawn_task(func() { return left(0) })
assert_equal(await await_task(task), [9])
assert_equal(left(1), [9])
assert_equal(right(1), [1])
"#,
        ),
    ];
    let mut failures = Vec::new();
    for (name, source) in cases {
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_task_reentry_{}_{}_{}",
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
            if !status.is_some_and(|status| status.success()) || output.trim() != "PASS" {
                failures.push(format!("{name}/{runtime}: {status:?}, {output:?}, {error}"));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
