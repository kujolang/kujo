use std::{fs, process::Command};

const MODULE: &str = r#"
shared_add_int("callback_module_initializations", 1)
export func compare_values(a, b) { return a - b }
"#;

fn check(body: &str, can_write: bool) {
    check_with_modules(body, can_write, &[]);
}

fn check_with_modules(body: &str, can_write: bool, modules: &[(&str, &str)]) {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("marker.kujo"), MODULE).unwrap();
    fs::write(
        directory.path().join("main.kujo"),
        format!(
            r#"
shared_set("callback_module_initializations", 0)
func compare(a, b) {{
    from marker import compare_values
    return compare_values(a, b)
}}
{body}
print("PASS")
"#
        ),
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        // Each mode starts with the same source; the reload case mutates it.
        fs::write(directory.path().join("marker.kujo"), MODULE).unwrap();
        for (name, source) in modules {
            fs::write(directory.path().join(name), source).unwrap();
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.current_dir(directory.path()).args([
            "run",
            "--untrusted",
            "--scheduler-no-timeout",
        ]);
        if can_write {
            command.arg("--allow-fs-write");
        }
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg("main.kujo").output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode}");
    }
}

#[test]
fn callback_imports_share_the_callers_unchanged_module_cache() {
    check(
        r#"
assert_equal(sort_by([4,3,2,1], compare), [1,2,3,4])
assert_equal(sort_by([3,1,2], compare), [1,2,3])
from marker import compare_values
assert_equal(compare_values(7,3), 4)
assert_equal(shared_get("callback_module_initializations"), 1)
"#,
        false,
    );
}

#[test]
fn nested_callbacks_return_the_same_module_cache_to_each_caller() {
    check(
        r#"
func outer(a, b) {
    from marker import compare_values
    assert_equal(sort_by([3,1,2], compare), [1,2,3])
    return compare_values(a, b)
}
assert_equal(sort_by([3,2,1], outer), [1,2,3])
assert_equal(sort_by([2,1], compare), [1,2])
assert_equal(shared_get("callback_module_initializations"), 1)
"#,
        false,
    );
}

#[test]
fn callback_errors_return_module_cache_to_the_caller() {
    check(
        r#"
func fail(a, b) {
    from marker import compare_values
    compare_values(a, b)
    throw("after import")
}
mut caught := false
try { sort_by([2,1], fail) }
except error { caught = contains(error.message, "after import") == 1 }
assert(caught, "callback error must propagate")
assert_equal(sort_by([3,2,1], compare), [1,2,3])
assert_equal(shared_get("callback_module_initializations"), 1)
"#,
        false,
    );
}

#[test]
fn callback_module_cache_still_reloads_changed_source() {
    let replacement =
        format!("{MODULE}\n# A different length guarantees changed source metadata.\n");
    let source = format!(
        r#"
assert_equal(sort_by([3,2,1], compare), [1,2,3])
write_file("marker.kujo", {}, true)
assert_equal(sort_by([3,2,1], compare), [1,2,3])
assert_equal(shared_get("callback_module_initializations"), 2)
"#,
        serde_json::to_string(&replacement).unwrap()
    );
    check(&source, true);
}

#[test]
fn callback_module_initializers_retain_capabilities_and_recover_after_denial() {
    check_with_modules(
        r#"
func denied(a, b) {
    from denied_module import compare_values
    return compare_values(a, b)
}
mut i := 0
while i < 2 {
    mut caught := false
    try { sort_by([2,1], denied) }
    except error { caught = contains(error.message, "Capability denied") == 1 }
    assert(caught, "module initializer must retain restricted capabilities")
    i += 1
}
assert_equal(sort_by([2,1], compare), [1,2])
assert_equal(shared_get("callback_module_initializations"), 1)
"#,
        false,
        &[
            (
                "denied_module.kujo",
                r#"
read_file("fixture.txt")
export func compare_values(a, b) { return a - b }
"#,
            ),
            ("fixture.txt", "fixture-only"),
        ],
    );
}

#[test]
fn callback_circular_import_errors_restore_loader_state() {
    check_with_modules(
        r#"
func cyclic(a, b) {
    from cycle_a import value_a
    return a - b
}
mut i := 0
while i < 2 {
    mut caught := false
    try { sort_by([2,1], cyclic) }
    except error {
        caught = contains(error.message, "Circular import detected: cycle_a -> cycle_b -> cycle_a") == 1
    }
    assert(caught, "callback import cycle diagnostic must retain the complete chain")
    i += 1
}
assert_equal(sort_by([2,1], compare), [1,2])
assert_equal(shared_get("callback_module_initializations"), 1)
"#,
        false,
        &[
            ("cycle_a.kujo", "from cycle_b import value_b\nexport let value_a := 1\n"),
            ("cycle_b.kujo", "from cycle_a import value_a\nexport let value_b := 2\n"),
        ],
    );
}
