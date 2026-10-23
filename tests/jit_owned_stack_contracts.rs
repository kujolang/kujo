use std::process::Command;

#[test]
fn native_callback_owned_result_survives_return_and_repeated_calls() {
    // Unroll calls so this probes whole-function compilation, independently of
    // loop compilation. The final call must run after the hot-call threshold.
    let mut source =
        String::from("func render(x) { return str(x) }\nfunc width(x) { return len(str(x)) }\n");
    for _ in 0..200 {
        source.push_str("assert_equal(render(17), \"17\")\n");
        source.push_str("assert_equal(width(17), 2)\n");
    }
    source.push_str("assert_equal(render(29), \"29\")\nprint(\"PASS\")\n");
    check(source, &["render", "width"]);
}

#[test]
fn native_owned_conditions_use_value_truthiness() {
    let mut source = String::from(
        "func empty(x) { if substr(str(x), 0, 0) { return 1 } return 0 }\n\
         func full(x) { if str(x) { return 1 } return 0 }\n\
         func either(x) { return substr(str(x), 0, 0) || str(x) }\n",
    );
    for _ in 0..200 {
        source.push_str("assert_equal(empty(17), 0)\nassert_equal(full(17), 1)\nassert_equal(either(17), true)\n");
    }
    source.push_str("print(\"PASS\")\n");
    check(source, &["empty", "full", "either"]);
}

#[test]
fn native_named_owned_values_survive_returns_and_callbacks() {
    let mut source = String::from(
        "let words := \"hello\"\nlet empty_text := \"\"\nlet numbers := [1, 2]\nlet mapping := {\"a\": 1}\n\
         func text(x) { return words }\nfunc array(x) { return numbers }\nfunc dict(x) { return mapping }\n\
         mut held := numbers\nfunc replace(x) { held = numbers return held }\n\
         func size(x) { return len(numbers) }\nfunc echo(x) { return x }\n\
         func relay(x) { return echo(numbers) }\nfunc truth(x) { if empty_text { return 1 } return 0 }\n",
    );
    for _ in 0..200 {
        source.push_str("assert_equal(text(1), words)\nassert_equal(array(1), numbers)\nassert_equal(dict(1), mapping)\nassert_equal(size(1), 2)\nassert_equal(relay(1), numbers)\nassert_equal(truth(1), 0)\nassert_equal(replace(1), numbers)\n");
    }
    source.push_str("mut copy := array(1)\ncopy[0] = 99\nassert_equal(numbers[0], 1)\nassert_equal(held[0], 1)\nprint(\"PASS\")\n");
    check(source, &["text", "array", "dict", "size", "relay", "truth", "replace"]);
}

fn check(source: String, functions: &[&str]) {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("owned_result.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let metrics = directory.path().join("metrics.json");
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(flag) = mode {
            command.arg(flag);
        }
        if mode == Some("--jit") {
            command.arg("--measurements").arg(&metrics);
            command.env("DEBUG_JIT", "1");
        }
        let output = command.arg(&file).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode:?}");
        if mode == Some("--jit") {
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(
                report["counters"]["jit_native_function_entries"].as_u64().unwrap()
                    >= 100 * functions.len() as u64
            );
            let stderr = String::from_utf8_lossy(&output.stderr);
            for name in functions {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
        }
    }
}
