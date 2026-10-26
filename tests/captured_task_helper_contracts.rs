//! Captured journals must follow lexical scopes through ordinary helper calls.
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn async_capture_journals_exclude_helper_parameters_and_restore_caller_writes() {
    let cases = [
        ("before_helper", "func shadow(a) { return a }", "b = 1 shadow(0)"),
        ("after_helper", "func shadow(a) { return a }", "shadow(0) b = 1"),
        (
            "nested_helper",
            "func inner(a) { return a } func shadow(value) { return inner(value) }",
            "shadow(0) b = 1",
        ),
        (
            "throwing_helper",
            "func shadow(a) { throw(\"helper\") }",
            "mut caught := false try { shadow(0) } except err { assert_equal(err.message, \"helper\") caught = true } assert(caught) b = 1",
        ),
        ("renamed_control", "func shadow(unrelated) { return unrelated }", "shadow(0) b = 1"),
    ];
    let mut failures = Vec::new();
    for (name, helper, action) in cases {
        let source = format!(
            r#"
shared_set("started", 0)
let left_gate := channel()
let right_gate := channel()
{helper}
func factory() {{
    mut a := 0
    mut b := 0
    async func apply_change(key) {{
        if key == 0 {{ a = 1 }}
        if key == 1 {{ {action} }}
        if key < 2 {{ shared_add_int("started", 1) }}
        if key == 0 {{ left_gate.receive() }}
        if key == 1 {{ right_gate.receive() }}
        return a + b
    }}
    return apply_change
}}
let apply_change := factory()
let left := apply_change(0)
let right := apply_change(1)
while shared_get("started") < 2 {{}}
left_gate.send(1)
await left
right_gate.send(1)
await right
print(await apply_change(2))
"#
        );
        for runtime in ["vm", "interpreter", "jit"] {
            let directory = std::env::temp_dir().join(format!(
                "kujo_task_helper_{}_{}_{}",
                std::process::id(),
                name,
                runtime
            ));
            fs::create_dir_all(&directory).unwrap();
            let script = directory.join("fixture.kujo");
            let stdout = directory.join("stdout");
            let stderr = directory.join("stderr");
            fs::write(&script, &source).unwrap();
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
            if !status.is_some_and(|status| status.success()) || output.trim() != "2" {
                failures.push(format!("{name}/{runtime}: {status:?}, {output:?}, {error}"));
            }
            fs::remove_dir_all(directory).unwrap();
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
