use std::process::Command;

#[test]
fn global_writes_preserve_snapshots_errors_and_replacement_root_order() {
    for mode in ["vm", "interpreter", "jit"] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
        cmd.args(["run", "tests/fixtures/global_indexed_write.kujo", "--untrusted"]);
        match mode {
            "interpreter" => {
                cmd.arg("--interpreter");
            }
            "jit" => {
                cmd.arg("--jit");
            }
            _ => {}
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "{mode}: {}", String::from_utf8_lossy(&out.stderr));
        // Index side effects complete before loading the destination root.
        let expected = "[42, 9, 9]";
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), expected, "{mode}");
    }
}

#[test]
fn malformed_call_counts_report_underflow_without_reserving_unbounded_storage() {
    use kujo::{
        bytecode::{BytecodeChunk, Constant, OpCode},
        vm::VM,
    };
    let mut chunk = BytecodeChunk::new();
    let index = chunk.add_constant(Constant::Int(1));
    chunk.emit(OpCode::LoadConst(index));
    chunk.emit(OpCode::Call(usize::MAX));
    let error = VM::new().execute(chunk).unwrap_err();
    assert!(error.contains("Stack underflow in Call args"), "{error}");
}

#[test]
fn cached_global_depth_checks_reject_growth_and_allow_repair() {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            use kujo::{
                compiler::Compiler,
                interpreter::{Interpreter, Value},
                lexer::tokenize,
                parser::Parser,
                vm::VM,
            };
            use std::sync::{Arc, Mutex};
            for jit in [false, true] {
                let mut deep = Value::Int(0);
                for _ in 0..kujo::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
                    deep = Value::array(vec![deep]);
                }
                let mut env = Interpreter::new().env;
                env.define("deep".into(), deep);
                let source = r#"
                mut data := [0]
                mut caught := 0
                try { data[0] = deep } except error { caught += 1 }
                assert_equal(caught, 1)
                assert_equal(data, [0])
                data[0] = [1]
                // Native host results can introduce deeper values outside this cache.
                data = push(data, deep)
                try { data[0] = [2] } except error { caught += 1 }
                assert_equal(caught, 2)
                assert_equal(data[0], [1])
                data[1] = 0
                data[0] = [2]
                assert_equal(data, [[2], 0])
                let immutable := [1]
                try { immutable[0] = 2 } except error { caught += 1 }
                assert_equal(caught, 3)
                assert_equal(immutable, [1])
            "#;
                let mut parser = Parser::new(tokenize(source).unwrap());
                let chunk = Compiler::new().compile(&parser.parse()).unwrap();
                let mut vm = VM::new();
                vm.set_globals(Arc::new(Mutex::new(env)));
                vm.set_jit_enabled(jit);
                vm.execute(chunk).unwrap();
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
