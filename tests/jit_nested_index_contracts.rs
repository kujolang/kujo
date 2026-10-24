use std::process::Command;

#[test]
fn native_nested_index_operands_preserve_types_effects_and_error_recovery() {
    let mut source = String::from(
        r#"mut before := 0
mut after := 0
func select(rows, i, j) { return rows[i][j] }
func identity(value) { return value }
func from_call(value, i) { return identity(value)[i] }
func effectful(rows, i, j) { before += 1 let result := rows[i][j] after += 1 return result }
let seed := [[7, true, null, 2.5, "héllo", [3,4], {"key":5}, bytes([0,255])]]
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(select(seed,0,0),7)\nassert_equal(from_call(seed,0),seed[0])\nassert_equal(effectful(seed,0,0),7)\n");
    }
    source.push_str(
        r#"mut i := 0
while i < len(seed[0]) { assert_equal(select(seed,0,i),seed[0][i]) i += 1 }
assert_equal(select(seed,0,-1),bytes([0,255]))
assert_equal(select(["héllo"],0,0),"h")
assert_equal(select([bytes([0,255])],0,-1),255)
assert_equal(select([{"key":5}],0,"key"),5)
mut target := "unchanged"
mut caught := 0
try { target = effectful(seed,0,99) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,"unchanged")
assert_equal(before,201)
assert_equal(after,200)
assert_equal(effectful(seed,0,1),true)
assert_equal(before,202)
assert_equal(after,201)
print("PASS")
"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested_index.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let output = command.arg(&path).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in ["select", "from_call", "effectful"] {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 300);
        }
    }
}
