use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_guarded_strings_preserve_success_fallback_and_rebinding() {
    let mut source = String::from(
        r#"
func looped(seed,tail) {
    mut value := seed
    mut i := 0
    while i < 4 { value = value + tail i += 1 }
    return value
}
func self_append(seed) { mut value := seed value = value + value return value }
func choose(flag,seed,tail) {
    mut value := seed
    if flag { value = value + tail } else { value = value + tail }
    return value
}
mut text := "seed"
func global_join(tail) { text = text + tail return text }
mut rhs_count := 0
func replacement() { rhs_count += 1 text = "replacement" return "!" }
func changed() { text = text + replacement() return text }
const fixed := "base"
func locked(tail) { fixed = fixed + tail }
mut caught := 0
let saved := text
"#,
    );
    for i in 0..200 {
        source.push_str("assert_equal(self_append(\"ab\"),\"abab\")\n");
        source.push_str("try { locked(\"!\") } except error { caught += 1 }\n");
        source.push_str("assert_equal(looped(\"λ\",\"雪\"),\"λ雪雪雪雪\")\nassert_equal(looped(3,2),11)\ntext = \"seed\"\nassert_equal(global_join(\"!\"),\"seed!\")\ntext = \"seed\"\nassert_equal(changed(),\"seed!\")\n");
        source.push_str(&format!("assert_equal(choose({},\"a\",\"b\"),\"ab\")\n", i % 2 == 0));
    }
    source.push_str(
        r#"
assert_equal(saved,"seed")
assert_equal(rhs_count,200)
assert_equal(caught,200)
assert_equal(fixed,"base")
assert_equal(looped("", ""),"")
print("PASS")
"#,
    );
    check(source, &["looped", "choose", "global_join", "changed", "locked", "self_append"]);
}

fn check(source: String, names: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guarded_strings.kujo");
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

#[test]
fn native_loop_string_guards_persist_locals_and_update_locked_globals() {
    check(
        r#"
mut text := ""
func once() {
    mut value := ""
    let saved := value
    let suffix := "x"
    mut i := 0
    while i < 200 {
        value = value + suffix
        text = text + suffix
        i += 1
    }
    assert_equal(len(value),200)
    assert_equal(value,text)
    assert_equal(saved,"")
}
once()
print("PASS")
"#
        .into(),
        &[],
    );
}
