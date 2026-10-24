use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_nested_writes_preserve_values_errors_and_evaluation_order() {
    let mut source = String::from(
        r#"
struct Holder { rows: any }
func write(seed,index,value) { mut state := seed state["rows"][index][0] = value return state }
func field(seed,value) { mut state := seed state.rows[0][0] = value return state }
func immutable(seed,value) { let state := seed state["rows"][0][0] = value }
mut state := {"rows": [[1,2]]}
func global_write(value) { state["rows"][0][0] = value }
const fixed := {"rows": [[1,2]]}
func const_write(value) { fixed["rows"][0][0] = value }
mut order := ""
func value_first() { order += "v" state = {"rows": [[7,8]]} return 9 }
func key_second() { order += "k" return "rows" }
func row_third() { order += "r" return 0 }
func ordered() { state[key_second()][row_third()][0] = value_first() }
let seed := {"rows": [[1,2]]}
let holder := Holder { rows: [[1,2]] }
mut caught := 0
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(write(seed,0,9),{"rows": [[9,2]]})
assert_equal(write(seed,-1,"snow"),{"rows": [["snow",2]]})
assert_equal(field(holder,true),Holder { rows: [[true,2]] })
global_write(5)
assert_equal(state,{"rows": [[5,2]]})
order = ""
ordered()
assert_equal(order,"vkr")
assert_equal(state,{"rows": [[9,8]]})
state = {"rows": [[1,2]]}
try { immutable(seed,9) } except error { caught += 1 }
try { const_write(9) } except error { caught += 1 }
"#,
        );
    }
    source.push_str(
        r#"
mut result := 17
try { result = write(seed,99,9) } except error { caught += 1 }
try { result = write({"rows": [1]},0,9) } except error { caught += 1 }
assert_equal(result,17)
assert_equal(caught,402)
assert_equal(seed,{"rows": [[1,2]]})
assert_equal(holder,Holder { rows: [[1,2]] })
assert_equal(fixed,{"rows": [[1,2]]})
assert_equal(write(seed,0,null),{"rows": [[null,2]]})
print("PASS")
"#,
    );
    check(source, &["write", "field", "immutable", "global_write", "const_write", "ordered"]);
}

#[test]
fn native_nested_loop_writes_preserve_local_global_snapshots() {
    check(
        r#"
mut global := {"rows": [[0,1]]}
let saved_global := global
func once() {
    mut local := {"rows": [[0,1]]}
    let saved := local
    mut i := 0
    while i < 200 {
        local["rows"][0][0] = i
        global["rows"][0][0] = i
        i += 1
    }
    return [local,saved]
}
assert_equal(once(),[{"rows": [[199,1]]},{"rows": [[0,1]]}])
assert_equal(global,{"rows": [[199,1]]})
assert_equal(saved_global,{"rows": [[0,1]]})
print("PASS")
"#
        .into(),
        &[],
    );
}

fn check(source: String, names: &[&str]) {
    check_modes(source, names, true);
}

fn check_modes(source: String, names: &[&str], expect_native: bool) {
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
            if !expect_native {
                assert!(!stderr.contains("Executing compiled loop"), "{stderr}");
                assert_eq!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap(), 0);
            } else if names.is_empty() {
                assert!(stderr.contains("Executing compiled loop"), "{stderr}");
                assert!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap() > 0);
            } else {
                assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
            }
        }
    }
}

#[test]
fn captured_path_loop_does_not_mutate_same_named_global() {
    check_modes(
        r#"
mut state := {"rows": [[999,1]]}
func outer() {
    mut state := {"rows": [[0,1]]}
    func work() {
        mut i := 0
        while i < 200 { state["rows"][0][0] = i i += 1 }
        return state
    }
    return work
}
let work := outer()
assert_equal(work(),{"rows": [[199,1]]})
assert_equal(state,{"rows": [[999,1]]})
print("PASS")
"#
        .into(),
        &[],
        false,
    );
}

#[test]
fn recursive_native_nested_writes_preserve_caller_snapshots() {
    check(
        r#"
        func visit(input,left,right) {
            mut state := input
            if left >= right { state[0][left] = state[0][left] + 1 return state }
            let middle := to_int((left + right) / 2)
            state = visit(state,left,middle)
            state = visit(state,middle + 1,right)
            return state
        }
        let seed := [range(0,512)]
        let result := visit(seed,0,511)
        assert_equal(result,[range(1,513)])
        assert_equal(seed,[range(0,512)])
        print("PASS")
        "#
        .into(),
        &["visit"],
    );
}
