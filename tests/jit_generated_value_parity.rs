use std::process::Command;

#[test]
fn generated_owned_branches_preserve_values_and_failure_effects_across_runtimes() {
    let values =
        ["17", "true", "null", "1.25", "\"héllo\"", "[1,2]", "{\"key\":7}", "bytes([0,255])"];
    let mut state = 0x4b554a4fu32;
    let mut source = String::new();
    for case in 0..24 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let first = values[(state as usize >> 8) % values.len()];
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let second = values[(state as usize >> 8) % values.len()];
        source.push_str(&format!(
            r#"
            mut before_{case} := 0
            mut after_{case} := 0
            func choose_{case}(items, k) {{
                before_{case} += 1
                mut result := items[0]
                if k > 0 {{ result = items[1] }}
                if k < 0 {{ result = items[99] }}
                after_{case} += 1
                return result
            }}
            let seed_{case} := [{first}, {second}]
        "#
        ));
        // Unroll calls so a compiled enclosing loop cannot mask function admission.
        for _ in 0..200 {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let branch = (state >> 16) & 1;
            let expected = if branch == 0 { first } else { second };
            source.push_str(&format!(
                "assert_equal(choose_{case}(seed_{case}, {branch}), {expected})\n"
            ));
        }
        source.push_str(&format!(r#"
            mut target_{case} := "unchanged"
            mut caught_{case} := 0
            try {{ target_{case} = choose_{case}(seed_{case}, -1) }} except error {{ caught_{case} += 1 }}
            assert_equal(target_{case}, "unchanged")
            assert_equal(caught_{case}, 1)
            assert_equal(before_{case}, 201)
            assert_equal(after_{case}, 200)
            assert_equal(choose_{case}(seed_{case}, 1), {second})
            assert_equal(before_{case}, 202)
            assert_equal(after_{case}, 201)
            assert_equal(seed_{case}, [{first}, {second}])
        "#));
    }
    source.push_str("print(\"PASS\")\n");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("generated.kujo");
    std::fs::write(&path, source).unwrap();
    let mut reference = None;
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
        if let Some(reference) = &reference {
            assert_eq!(&output.stdout, reference);
        } else {
            reference = Some(output.stdout);
        }
        if mode == Some("--jit") {
            for case in 0..24 {
                assert!(
                    stderr.contains(&format!("Successfully compiled function 'choose_{case}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 2400);
        }
    }
}
