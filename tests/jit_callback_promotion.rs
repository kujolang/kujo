use std::process::Command;

#[test]
fn hot_dispatch_retains_exception_closure_and_loop_behavior() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("dispatch.kujo");
    std::fs::write(&file, r#"
        func guarded(input) { try { return input[0] } except error { return 41 } }
        func counted(input) { mut n := 0; mut i := 0; while i < 3 { n += input[0]; i += 1 }; return n }
        func capture(offset) { return func(x) { return x + offset } }
        func invoke(f, x) { return f(x) }
        func relay(input) { return guarded(input) }
        let saved := [7]
        let add := capture(5)
        mut i := 0
        while i < 180 {
            assert_equal(relay(saved), 7)
            assert_equal(relay([]), 41)
            assert_equal(counted(saved), 21)
            assert_equal(invoke(add, i), i + 5)
            assert_equal(saved, [7])
            i += 1
        }
        print("PASS")
    "#).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let output = command.arg(&file).output().unwrap();
        assert!(output.status.success(), "{mode:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
    }
}

#[test]
fn quicksort_promotes_pivot_helper_without_changing_snapshots_or_errors() {
    // Use the actual port kernel, so the coverage test cannot silently diverge.
    let port = include_str!("../benchmarks/langarena/port.kujo");
    let kernel = port.split("func quick_sort(input, low, high) {").nth(1).unwrap();
    let kernel = kernel.split("func merge_sort(").next().unwrap();
    let mut source = format!(
        r#"
        func idiv(a, b) {{ return to_int(a / b) }}
        func quick_sort(input, low, high) {{{kernel}
        let original := [9, -2, 9, 0, 4, -2, 7, 1]
        mut i := 0
        while i < 160 {{
            assert_equal(quick_sort(original, 0, 7), [-2,-2,0,1,4,7,9,9])
            assert_equal(original, [9,-2,9,0,4,-2,7,1])
            i += 1
        }}
        assert_equal(quick_sort([], 0, -1), [])
        assert_equal(quick_sort([3], 0, 0), [3])
        mut caught := 0
        mut saved := [42]
        try {{ saved = quick_sort(original, 0, 99) }} except error {{ caught += 1 }}
        assert_equal(caught, 1)
        assert_equal(saved, [42])
        assert_equal(original, [9,-2,9,0,4,-2,7,1])
        assert_equal(quick_sort([2,1], 0, 1), [1,2])
        "#
    );
    // Entire-output checks across distinct recursion shapes. The same source
    // runs in all three runtimes, and saved snapshots must survive every call.
    let mut seed = 42_u64;
    for size in [0, 1, 2, 17, 64, 127] {
        let random: Vec<i64> = (0..size)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 8) % 31) as i64 - 15
            })
            .collect();
        let mut sorted = random.clone();
        sorted.sort();
        let reverse: Vec<_> = sorted.iter().copied().rev().collect();
        for (case, input) in
            [random, sorted.clone(), reverse, vec![7; size]].into_iter().enumerate()
        {
            let mut expected = input.clone();
            expected.sort();
            source.push_str(&format!(
                "let snapshot_{size}_{case} := {input:?}; assert_equal(quick_sort(snapshot_{size}_{case}, 0, {}), {expected:?}); assert_equal(snapshot_{size}_{case}, {input:?})\n",
                size as i64 - 1
            ));
        }
    }
    source.push_str("print(\"PASS\")\n");
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("quicksort.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1");
        }
        let output = command.arg(&file).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'quick_sort'"), "{stderr}");
            assert!(
                stderr.contains("Successfully compiled function 'idiv' from native callback"),
                "{stderr}"
            );
        }
    }
}
