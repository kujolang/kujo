use std::fs;
use std::process::Command;

fn run_case(name: &str, source: &str, restricted: bool) -> Vec<std::process::Output> {
    let dir = std::env::temp_dir().join(format!("kujo_callback_{}_{}", std::process::id(), name));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("bridge.kujo"),
        r#"
export func invoke(callback, value) { return callback(value) }
export func twice(callback) { return [callback(1), callback(2)] }
export func mapped(callback) { return map([1,2], callback) }
export async func invoke_async(callback, value) { return callback(value) }
export func await_callback(callback, value) { let result := await callback(value); return result["value"] }
"#,
    )
    .unwrap();
    fs::write(dir.join("main.kujo"), source).unwrap();
    let outputs = [false, true]
        .into_iter()
        .map(|interpreter| {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
            cmd.current_dir(&dir).args(["run", "main.kujo"]);
            if interpreter {
                cmd.arg("--interpreter");
            }
            if restricted {
                cmd.arg("--untrusted");
            }
            cmd.output().unwrap()
        })
        .collect();
    fs::remove_dir_all(dir).unwrap();
    outputs
}

#[test]
fn imported_callbacks_preserve_results_captures_and_globals() {
    let outputs = run_case(
        "values",
        r#"
from bridge import invoke, twice, mapped
mut global := 40
func make() {
    mut count := 0
    return func(value) { count = count + value; return count + global }
}

global = 41
let callback := make()
print(to_json(twice(callback)))
print(to_json(mapped(func(value) { return {"value":value,"nested":[true, null]} })))
print(invoke(func(value) { try { throw("caught") } except err { return value + 1 } }, 4))
"#,
        false,
    );
    for output in outputs {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let out = String::from_utf8(output.stdout).unwrap();
        assert_eq!(out.trim(), "[42,44]\n[{\"nested\":[true,null],\"value\":1},{\"nested\":[true,null],\"value\":2}]\n5");
    }
}

#[test]
fn imported_callback_errors_are_catchable_and_do_not_poison_next_call() {
    for output in run_case(
        "errors",
        r#"
from bridge import invoke
try { invoke(func(value) { throw("callback-failure") }, 1) } except err { print("caught") }
print(invoke(func(value) { return value + 1 }, 2))
"#,
        false,
    ) {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "caught\n3");
    }
}

#[test]
fn imported_async_function_preserves_callback_globals() {
    for output in run_case(
        "async",
        r#"
from bridge import invoke_async
let offset := 40
print(await invoke_async(func(value) { return value + offset }, 2))
"#,
        false,
    ) {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    }
}

#[test]
fn imported_function_can_await_vm_callback_results() {
    for output in run_case(
        "async_callback",
        r#"
from bridge import await_callback
async func callback(value) { return {"value":value + 40} }
print(await_callback(callback, 2))
"#,
        false,
    ) {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    }
}

#[test]
fn byte_length_is_exact_in_both_restricted_runtimes() {
    for output in run_case(
        "byte_length",
        r#"
print(to_json([byte_length(""),byte_length("abc"),byte_length("é"),byte_length("😀"),byte_length("aé😀"),byte_length(bytes([0,128,255]))]))
try { byte_length(null) } except err { print("type-denied") }
try { byte_length("a","b") } except err { print("arity-denied") }
"#,
        true,
    ) {
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "[0,3,2,4,7,3]\ntype-denied\narity-denied"
        );
    }
}

#[test]
fn imported_callback_arity_is_enforced() {
    for output in run_case(
        "arity",
        r#"
from bridge import invoke
invoke(func() { return 42 }, 1)
"#,
        false,
    ) {
        assert_eq!(output.status.code(), Some(4));
        assert!(String::from_utf8_lossy(&output.stderr).contains("argument"));
    }
}

#[test]
fn imported_callback_cannot_gain_capabilities() {
    for output in run_case(
        "denied",
        r#"
from bridge import invoke
invoke(func(value) { return read_file(value) }, "bridge.kujo")
"#,
        true,
    ) {
        assert_eq!(output.status.code(), Some(4));
        assert!(
            String::from_utf8_lossy(&output.stderr).to_lowercase().contains("capability"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn cross_runtime_recursion_fails_without_aborting_process() {
    for output in run_case(
        "recursion",
        r#"
from bridge import invoke
func recurse(value) { return invoke(value[0], [value[0],value[1] + 1]) }
recurse([recurse,0])
"#,
        false,
    ) {
        assert_eq!(output.status.code(), Some(4), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("depth"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
