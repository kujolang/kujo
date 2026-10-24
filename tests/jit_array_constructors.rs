use std::process::Command;

#[test]
fn native_array_literals_preserve_order_types_and_value_semantics() {
    let mut source = String::from(
        r#"
mut effects := 0
func mark(x) { effects += 1 return x }
func build(a,b) { return [mark(a),mark(b),true,null,2.5,"hé",bytes([0,255])] }
func empty() { return [] }
func wrap(x) { return [x] }
func risky(d) { return [mark(1),10/d,mark(2)] }
let seed := [7,8]
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(build(seed,9),[seed,9,true,null,2.5,\"hé\",bytes([0,255])])\nassert_equal(empty(),[])\nassert_equal(wrap(seed),[seed])\n");
    }
    source.push_str(
        r#"
assert_equal(effects,400)
mut result := build(seed,11)
result[0][0] = 99
assert_equal(seed,[7,8])
assert_equal(result[0],[99,8])
assert_equal(effects,402)
assert_equal(build(false,{"k":3}),[false,{"k":3},true,null,2.5,"hé",bytes([0,255])])
assert_equal(effects,404)
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(risky(2),[1,5,2])\n");
    }
    source.push_str(
        r#"
assert_equal(effects,804)
mut target := [17]
mut caught := 0
try { target = risky(0) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,[17])
assert_equal(effects,805)
assert_equal(risky(2),[1,5,2])
assert_equal(effects,807)
print("PASS")
"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("arrays.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
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
            for name in ["build", "empty", "wrap", "risky"] {
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
