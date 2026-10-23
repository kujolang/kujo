use std::process::Command;

#[test]
fn optimized_and_unoptimized_bytecode_keep_fusion_and_fallback_balanced() {
    use kujo::{
        bytecode::{Constant, OpCode},
        compiler::Compiler,
        interpreter::Value,
        lexer,
        parser::Parser,
        vm::VM,
    };
    for optimize in [false, true] {
        for start in ["0", "9", "0.5", "9223372036854775805"] {
            let limit = if start.len() > 3 { "9223372036854775807" } else { "3" };
            let expected = match start {
                "0" => 3,
                "9" => 0,
                "0.5" => 3,
                _ => 2,
            };
            let source = format!("func build() {{ mut text := \"\" mut i := {start} let limit := {limit} while i < limit {{ text = text + \"x\" i = i + 1 }} return len(text) }} return build()");
            let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
            let chunk =
                Compiler::new().compile_with_optimization(&parser.parse(), optimize).unwrap();
            let function = chunk
                .constants
                .iter()
                .find_map(|constant| match constant {
                    Constant::Function(chunk) => Some(chunk),
                    _ => None,
                })
                .unwrap();
            assert!(function
                .instructions
                .iter()
                .any(|op| matches!(op, OpCode::AppendConstCharUntilLocalInPlace(..))));
            assert!(function.instructions.iter().any(|op| matches!(op, OpCode::JumpBack(_))));
            let mut vm = VM::new();
            vm.set_jit_enabled(false);
            assert!(matches!(vm.execute(chunk).unwrap(), Value::Int(n) if n == expected));
        }
    }
}

fn check(source: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("append.kujo");
    std::fs::write(&path, format!("{source}\nprint(\"PASS\")\n")).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
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

#[test]
fn skipped_loop_preserves_counter_and_does_not_validate_target() {
    check(
        r#"
        func skipped(start, end) {
            mut text := 17
            mut i := start
            let limit := end
            while i < limit { text = text + "x" i = i + 1 }
            assert_equal(i, start)
            assert_equal(text, 17)
        }
        skipped(9, 3)
        skipped(3, 3)
        skipped(9.5, 3.5)
    "#,
    );
}

#[test]
fn fractional_counter_preserves_original_iteration_semantics() {
    check(
        r#"
        func fractional() {
            mut text := ""
            mut i := 0.5
            let limit := 3.0
            while i < limit { text = text + "x" i = i + 1 }
            assert_equal(text, "xxx")
            assert_equal(i, 3.5)
        }
        fractional()
    "#,
    );
}

#[test]
fn unicode_growth_and_snapshot_isolation_survive_multiple_batches() {
    check(
        r#"
        func grow() {
            mut text := "seed"
            let snapshot := text
            mut i := -3
            let limit := 10000
            while i < limit { text = text + "é" i = i + 1 }
            assert_equal(i, 10000)
            assert_equal(text, "seed" + repeat("é", 10003))
            assert_equal(snapshot, "seed")
        }
        grow()
    "#,
    );
}

#[test]
fn readonly_bindings_and_invalid_target_errors_are_catchable() {
    check(
        r#"
        func immutable_counter() {
            mut text := ""
            let i := 0
            let limit := 3
            mut caught := 0
            try { while i < limit { text = text + "x" i = i + 1 } }
            except error { caught += 1 }
            assert_equal(caught, 1)
            assert_equal(text, "x")
            assert_equal(i, 0)
        }
        func immutable_text() {
            const text := ""
            mut i := 0
            let limit := 3
            mut caught := 0
            try { while i < limit { text = text + "x" i = i + 1 } }
            except error { caught += 1 }
            assert_equal(caught, 1)
            assert_equal(i, 0)
            assert_equal(text, "")
        }
        func bad_target() {
            mut text := 17
            mut i := 0
            let limit := 3
            mut caught := 0
            try { while i < limit { text = text + "x" i = i + 1 } }
            except error { caught += 1 }
            assert_equal(caught, 1)
            assert_equal(i, 0)
        }
        func readonly_constant_appends() {
            let text := "seed"
            const other := "seed"
            mut caught := 0
            try { text = text + "x" } except error { caught += 1 }
            try { other = other + "suffix" } except error { caught += 1 }
            assert_equal(caught, 2)
            assert_equal(text, "seed")
            assert_equal(other, "seed")
        }
        immutable_counter()
        immutable_text()
        bad_target()
        readonly_constant_appends()
    "#,
    );
}
