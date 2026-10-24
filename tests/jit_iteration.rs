use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_iteration_preserves_values_exits_snapshots_and_lazy_generators() {
    let mut source = String::from(
        r#"
func sum(values) { mut total := 0 for item in values { total += item } return total }
func via(values) { return sum(values) }
func first(values) { for item in values { return item } return null }
func collect(values) { mut result := [] for item in values { result = push(result,item) } return result }
func exits(values) { mut total := 0 for item in values { if item == 2 { continue } if item == 4 { break } total += item } return total }
func nested(values) { mut total := 0 for row in values { for item in row { total += item } } return total }
func snapshot(seed) { mut values := seed mut total := 0 for item in values { values = push(values,99) total += item } return [total,values] }
mut effects := 0
func* produce() { effects += 1 yield 7 effects += 1 yield 9 }
func* fail() { effects += 1 yield 1 effects += 1 missing }
mut caught := 0
let mixed := [null,true,1.25,"雪",[1,2],{"a":7},bytes([255])]
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(sum([1,2,3]),6)
assert_equal(via([1,2,3]),6)
assert_equal(sum(bytes([0,127,255])),382)
assert_equal(sum([]),0)
assert_equal(first([]),null)
assert_equal(first(mixed),null)
assert_equal(collect(mixed),mixed)
assert_equal(exits([1,2,3,4,5]),4)
assert_equal(nested([[1,2],[],[3]]),6)
assert_equal(snapshot([1,2]),[3,[1,2,99,99]])
assert_equal(first(produce()),7)
try { sum(fail()) } except error { caught += 1 }
"#,
        );
    }
    source.push_str("assert_equal(caught,200)\nassert_equal(effects,600)\nprint(\"PASS\")\n");
    check(source, &["sum", "via", "first", "collect", "exits", "nested", "snapshot"]);
}

#[test]
fn native_loop_iteration_preserves_live_locals_and_generator_resume() {
    check(
        r#"
mut effects := 0
func* produce() { mut i := 0 while i < 200 { effects += 1 yield i i += 1 } }
func once(values) { mut total := 0 for item in values { total += item } return total }
func later(values) { for item in values { if item == 150 { return item } } return -1 }
func while_return() { mut i := 0 while i < 200 { if i == 150 { return i } i += 1 } return -1 }
func owned_return() { mut i := 0 while i < 200 { if i == 150 { return [i,"snow"] } i += 1 } return [] }
func skipped(values) { mut total := 0 for item in values { if item == 150 { continue } total += item } return total }
func stopped(values) { mut total := 0 for item in values { if item == 150 { break } total += item } return total }
assert_equal(later(range(0,200)),150)
assert_equal(while_return(),150)
assert_equal(owned_return(),[150,"snow"])
assert_equal(skipped(range(0,200)),19750)
assert_equal(stopped(range(0,200)),11175)
assert_equal(once(range(0,200)),19900)
assert_equal(once(produce()),19900)
assert_equal(effects,200)
print("PASS")
"#
        .into(),
        &[],
    );
}

fn check(source: String, names: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("iteration.kujo");
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
