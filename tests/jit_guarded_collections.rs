use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_collection_guards_preserve_aliases_replacement_and_shadowing() {
    let mut source = String::from(
        r#"
func collect(seed,item) { mut value := seed value = push(value,item) return value }
func append_item(seed,item) { mut value := seed value = append(value,item) return value }
func join_bytes(seed,tail) { mut value := seed value = concat(value,tail) return value }
func self_push(seed) { mut value := seed value = push(value,value) return value }
func custom(seed,item) { return [item,seed] }
func shadow(operation,seed,item) { let push := operation mut value := seed value = push(value,item) return value }
mut items := [1]
mut effects := 0
func replacement() { effects += 1 items = [9] return 2 }
func changed() { items = push(items,replacement()) return items }
func global_push(item) { items = push(items,item) return items }
const fixed := [1]
func locked(item) { fixed = push(fixed,item) }
let seed := [1]
let data := bytes([0,255])
mut caught := 0
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(collect(seed,2),[1,2])
assert_equal(append_item(seed,2),[1,2])
assert_equal(self_push(seed),[1,[1]])
assert_equal(join_bytes(data,data),bytes([0,255,0,255]))
assert_equal(shadow(custom,seed,2),[2,[1]])
items = [1]
assert_equal(changed(),[1,2])
items = [1]
assert_equal(global_push(2),[1,2])
try { locked(2) } except error { caught += 1 }
"#,
        );
    }
    source.push_str(
        r#"
assert_equal(seed,[1])
assert_equal(data,bytes([0,255]))
assert_equal(fixed,[1])
assert_equal(caught,200)
assert_equal(effects,200)
print("PASS")
"#,
    );
    check(
        source,
        &[
            "collect",
            "append_item",
            "join_bytes",
            "self_push",
            "shadow",
            "changed",
            "global_push",
            "locked",
        ],
    );
}

#[test]
fn native_loop_collection_guards_persist_local_and_global_values() {
    check(
        r#"
mut items := []
mut data := bytes([])
func once() {
    mut value := []
    mut buffer := bytes([])
    let saved := value
    let saved_buffer := buffer
    let tail := bytes([0,255])
    mut i := 0
    while i < 200 {
        value = push(value,i)
        items = push(items,i)
        buffer = concat(buffer,tail)
        data = concat(data,tail)
        i += 1
    }
    assert_equal(len(value),200)
    assert_equal(len(buffer),400)
    assert_equal(value,items)
    assert_equal(buffer,data)
    assert_equal(saved,[])
    assert_equal(saved_buffer,bytes([]))
}
once()
print("PASS")
"#
        .into(),
        &[],
    );
}

fn check(source: String, names: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guarded_collections.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if names.is_empty() {
            command.env("DISABLE_FUNCTION_JIT", "1").env("DEBUG_JIT_LOOP", "1");
        }
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let mut child =
            command.arg(&path).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("{mode:?} timed out: {}", String::from_utf8_lossy(&output.stderr));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in names {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            if names.is_empty() {
                assert!(stderr.contains("Executing compiled loop"), "{stderr}");
                assert!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap() > 0);
            } else {
                assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
            }
        }
    }
}
