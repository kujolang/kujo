use std::process::Command;

fn run(source: &str, runtime: &str) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("returned_try.kujo");
    std::fs::write(&path, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", "--untrusted"]);
    if runtime != "vm" {
        command.arg(format!("--{runtime}"));
    }
    command.arg(path).output().unwrap()
}

#[test]
fn returned_try_handlers_do_not_intercept_later_uncaught_errors() {
    for runtime in ["vm", "interpreter", "jit"] {
        for body in ["return 7", "return"] {
            let source = format!(
                "func guarded() {{ try {{ {body} }} except error {{ return -1 }} }}\n\
                 let value := guarded()\nassert(false, \"later caller failure\")"
            );
            let output = run(&source, runtime);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(4), "{runtime}: {stderr}");
            assert!(stderr.contains("later caller failure"), "{runtime}: {stderr}");
            assert!(!stderr.contains("Duplicate declaration"), "{runtime}: {stderr}");
        }
    }
}

#[test]
fn returning_from_nested_try_preserves_the_callers_handler_and_snapshots() {
    let source = r#"
        func guarded(n, input) {
            mut values := input
            try {
                try {
                    if n == 0 { return values }
                    values = push(values, n)
                    return guarded(n - 1, values)
                } except inner { throw("unexpected inner catch") }
            } except outer { throw("unexpected outer catch") }
        }
        let original := [9]
        mut caught := 0
        mut i := 0
        while i < 40 {
            try {
                assert_equal(guarded(3, original), [9, 3, 2, 1])
                throw("caller failure")
            } except caller { assert_equal(caller.message, "caller failure") caught += 1 }
            assert_equal(original, [9])
            i += 1
        }
        assert_equal(caught, 40)
        assert_equal(guarded(1, original), [9, 1])
    "#;
    for runtime in ["vm", "interpreter", "jit"] {
        let output = run(source, runtime);
        assert!(output.status.success(), "{runtime}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn loop_exits_discard_only_the_handlers_they_leave() {
    for runtime in ["vm", "interpreter", "jit"] {
        for exit in ["break", "continue"] {
            let source = format!(
                r#"
                mut i := 0
                while i < 3 {{
                    i += 1
                    try {{ {exit} }} except stale {{ throw("stale catch") }}
                }}
                assert(false, "expected after loop")
                "#
            );
            let output = run(&source, runtime);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(4), "{runtime}/{exit}: {stderr}");
            assert!(stderr.contains("expected after loop"), "{runtime}/{exit}: {stderr}");
        }
        let output = run(
            r#"
            mut caught := 0
            try {
                for item in [1, 2, 3] {
                    try { if item == 1 { continue } break }
                    except stale { throw("stale for handler") }
                }
                throw("outer expected")
            } except outer {
                assert_equal(outer.message, "outer expected")
                caught += 1
            }
            try {
                loop {
                    try { break } except stale { throw("stale loop handler") }
                }
                throw("second expected")
            } except outer {
                assert_equal(outer.message, "second expected")
                caught += 1
            }
            assert_equal(caught, 2)
            "#,
            runtime,
        );
        assert!(output.status.success(), "{runtime}: {}", String::from_utf8_lossy(&output.stderr));
    }
}
