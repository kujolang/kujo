use std::process::Command;

#[test]
fn native_struct_construction_preserves_names_types_aliases_and_effects() {
    let mut source = String::from(
        r#"
struct Pair { left: any, right: any }
struct Empty {}
struct Node { value: int, next: any }
mut effects := 0
func mark(x) { effects += 1 return x }
func build(a,b) { return Pair { left: mark(a), right: mark(b) } }
func empty() { return Empty {} }
func risky(d) { return Pair { left: mark(1), right: 10/d } }
func chain(n) {
 if n == 0 { return Node { value: 0, next: null } }
 return Node { value: n, next: chain(n-1) }
}
let seed := [7,8]
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(build(seed,true),Pair { left: seed, right: true })\nassert_equal(empty(),Empty {})\nassert_equal(risky(2),Pair { left: 1, right: 5 })\n");
    }
    for _ in 0..200 {
        source.push_str("assert_equal(chain(3),Node { value: 3, next: Node { value: 2, next: Node { value: 1, next: Node { value: 0, next: null } } } })\n");
    }
    source.push_str(
        r#"
assert_equal(effects,600)
mut value := build(seed,{"k":bytes([0,255])})
let snapshot := value
value.left[0] = 99
assert_equal(seed,[7,8])
assert_equal(snapshot.left,[7,8])
assert_equal(value.left,[99,8])
assert_equal(value.right,{"k":bytes([0,255])})
assert_equal(build(null,2.5),Pair { left: null, right: 2.5 })
assert_equal(effects,604)
mut target := "unchanged"
mut caught := 0
try { target = risky(0) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(target,"unchanged")
assert_equal(effects,605)
assert_equal(risky(2),Pair { left: 1, right: 5 })
assert_equal(effects,606)
print("PASS")
"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("structs.kujo");
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
            for name in ["build", "empty", "risky", "chain"] {
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
