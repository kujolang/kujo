use std::process::Command;

#[test]
fn quicksort_promotes_pivot_helper_without_changing_snapshots_or_errors() {
    // Use the actual port kernel, so the coverage test cannot silently diverge.
    let port = include_str!("../benchmarks/langarena/port.kujo");
    let kernel = port.split("func quick_sort(input, low, high) {").nth(1).unwrap();
    let kernel = kernel.split("func merge_sort(").next().unwrap();
    let source = format!(
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
        print("PASS")
        "#
    );
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
