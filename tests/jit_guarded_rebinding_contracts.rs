use std::process::Command;

#[test]
fn guarded_rebinding_preserves_evaluation_aliases_and_shadowed_calls() {
    let mut source = String::from(
        r#"
func join(seed,tail) { mut value := seed value = value + tail return value }
func collect(seed,item) { mut value := seed value = push(value,item) return value }
func custom(seed,item) { return [item,seed] }
func shadow(operation,seed,item) {
    let push := operation
    mut value := seed
    value = push(value,item)
    return value
}
let seed := [1]
"#,
    );
    for _ in 0..200 {
        source.push_str("assert_equal(join(\"λ\",\"雪\"),\"λ雪\")\nassert_equal(collect(seed,2),[1,2])\nassert_equal(shadow(custom,seed,2),[2,[1]])\n");
    }
    source.push_str(
        r#"
assert_equal(seed,[1])
mut text := "seed"
func change_text() { text = "replaced" return "!" }
text = text + change_text()
assert_equal(text,"seed!")
mut items := [1]
let saved := items
func change_items() { items = [9] return 2 }
items = push(items,change_items())
assert_equal(items,[1,2])
assert_equal(saved,[1])
mut effects := 0
func fail() { effects += 1 return 10/0 }
mut caught := 0
try { text = text + fail() } except error { caught += 1 }
try { items = push(items,fail()) } except error { caught += 1 }
assert_equal(caught,2)
assert_equal(effects,2)
assert_equal(text,"seed!")
assert_equal(items,[1,2])
print("PASS")
"#,
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guarded_rebinding.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}
