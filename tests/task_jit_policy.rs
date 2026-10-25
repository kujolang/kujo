use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(source: &str, mode: &str) -> Option<serde_json::Value> {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("task.kujo");
    let stdout = directory.path().join("stdout");
    let stderr = directory.path().join("stderr");
    let metrics = directory.path().join("metrics.json");
    fs::write(&script, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
    if mode != "vm" {
        command.arg(format!("--{mode}"));
    }
    if mode != "interpreter" {
        command.arg("--measurements").arg(&metrics);
    }
    let mut child = command
        .arg(script)
        .stdout(Stdio::from(File::create(&stdout).unwrap()))
        .stderr(Stdio::from(File::create(&stderr).unwrap()))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "{mode}: task cancellation exceeded deadline: {}",
                fs::read_to_string(stderr).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success(), "{mode}: {}", fs::read_to_string(stderr).unwrap());
    assert_eq!(fs::read_to_string(stdout).unwrap().trim(), "PASS", "{mode}");
    (mode != "interpreter").then(|| serde_json::from_slice(&fs::read(metrics).unwrap()).unwrap())
}

#[test]
fn async_detached_and_callback_work_inherit_jit_only_when_requested() {
    let kernel = r#"
        func kernel(n) {
            mut sum := 0
            mut i := 0
            while i < n { sum += i i += 1 }
            return sum
        }
    "#;
    for body in [
        "async func worker(n) { return kernel(n) } assert_equal(await parallel_map([20000,20000], worker, 2), [199990000,199990000])",
        "assert_equal(map([20000,20000], kernel), [199990000,199990000])",
        "func replacement(c) { assert_equal(kernel(20000),199990000) return \"x\" } assert_equal(regex_replace_with(\"aa\",\"a\",replacement),\"xx\")",
        "shared_set(\"done\",0) spawn { func inner(n) { mut sum := 0 mut i := 0 while i < n { sum += i i += 1 } return sum } let value := inner(20000) shared_set(\"done\",value) } while shared_get(\"done\") == 0 {} assert_equal(shared_get(\"done\"),199990000)",
    ] {
        for mode in ["vm", "interpreter", "jit"] {
            if mode == "jit" && !cfg!(feature = "runtime-jit") { continue; }
            if let Some(report) = run(&format!("{kernel}\n{body}\nprint(\"PASS\")"), mode) {
                let entries = report["counters"]["jit_native_function_entries"].as_u64().unwrap();
                if mode == "jit" {
                    // No kernel call runs in the parent: entries prove worker/bridge execution.
                    assert!(entries > 0, "{body}: {report}");
                } else { assert_eq!(entries,0,"JIT enabled without opt-in"); }
            }
        }
    }
}

#[test]
fn native_task_cancellation_crosses_callback_bridges_and_recovers_admission() {
    for callable in ["spin", "bridge", "regex_bridge"] {
        let source = format!(
            r#"
            shared_set("started", 0)
            func spin() {{
                shared_add_int("started", 1)
                mut i := 0
                while i < 9223372036854775807 {{ i += 1 }}
                return i
            }}
            func callback(unused) {{ return spin() }}
            func bridge() {{ return map([0], callback) }}
            func replacement(unused) {{ spin() return "x" }}
            func regex_bridge() {{ return regex_replace_with("a", "a", replacement) }}
            mut iteration := 0
            while iteration < 40 {{
                mut handle := null
                loop {{
                    try {{ handle = spawn_task({callable}) break }}
                    except error {{
                        assert(contains(error.message, "Language task limit") == 1,
                            "unexpected admission error")
                    }}
                }}
                while (shared_get("started") <= iteration) {{}}
                assert(cancel_task(handle), "cancellation must win")
                mut caught := 0
                try {{ await await_task(handle) }} except error {{ caught += 1 }}
                try {{ await await_task(handle) }} except error {{ caught += 1 }}
                assert_equal(caught,2)
                iteration += 1
            }}
            let gate := channel()
            shared_set("ready", 0)
            func sentinel() {{
                shared_add_int("ready", 1)
                gate.receive()
                return 42
            }}
            mut handles := []
            while len(handles) < 16 {{
                try {{
                    let handle := spawn_task(sentinel)
                    handles = push(handles, handle)
                }} except error {{
                    assert(contains(error.message, "Language task limit") == 1,
                        "unexpected admission error")
                }}
            }}
            while shared_get("ready") < 16 {{}}
            for handle in handles {{ gate.send(1) }}
            for handle in handles {{ assert_equal(await await_task(handle),42) }}
            print("PASS")
        "#
        );
        for mode in ["vm", "interpreter", "jit"] {
            if mode == "jit" && !cfg!(feature = "runtime-jit") {
                continue;
            }
            if let Some(report) = run(&source, mode) {
                let counts = &report["counters"];
                assert_eq!(counts["tasks_admitted"], 56, "{mode}/{callable}: {report}");
                assert_eq!(counts["task_cancellations_published"], 40);
                // All 16 sentinel bodies were simultaneously admitted and blocked.
                // The 16-slot pool therefore had to release all 40 cancelled bodies
                // before the sentinels were unblocked; no timing sleep is needed.
                assert!(counts["task_bodies_exited"].as_u64().unwrap() >= 40, "{report}");
                let entries = counts["jit_native_function_entries"].as_u64().unwrap();
                if mode == "jit" {
                    assert!(entries >= 40, "native workers were not entered: {report}");
                } else {
                    assert_eq!(entries, 0);
                }
            }
        }
    }
}
