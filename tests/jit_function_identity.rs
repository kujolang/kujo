use std::{fs, process::Command};

fn check(source: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("identity.kujo");
    fs::write(&path, source).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        if mode == "jit" && !cfg!(feature = "runtime-jit") {
            continue;
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS", "{mode}");
    }
}

#[test]
fn hot_same_name_functions_keep_distinct_code() {
    check(include_str!("../benchmarks/langarena/probes/jit_function_identity.kujo"));
}

#[test]
fn native_callers_dispatch_same_name_functions_by_identity() {
    check(
        r#"
func factory() {
    func read(x) { return x + 1 }
    let before := read
    func read(x) { return x + 2 }
    return [before, read]
}

func invoke(f, x) { return f(x) }
let readers := factory()
mut i := 0
while i < 200 {
    assert_equal(invoke(readers[0], 5), 6)
    i += 1
}
assert_equal(invoke(readers[1], 5), 7)
assert_equal(invoke(readers[0], 5), 6)
print("PASS")
"#,
    );
}

#[cfg(feature = "runtime-jit")]
#[test]
fn public_name_inspection_retains_latest_function_metadata() {
    use kujo::{
        compiler::Compiler, interpreter::Value, jit::JitCompiler, lexer, parser::Parser, vm::VM,
    };
    let mut jit = JitCompiler::new().unwrap();
    assert!(jit.get_fn_info("read").is_none());
    for (source, arity) in [
        ("func read(x) { return x + 1 } return read", 1),
        ("func read(x, y) { return x + y } return read", 2),
    ] {
        let mut parser = Parser::new(lexer::tokenize(source).unwrap());
        let script = Compiler::new().compile(&parser.parse()).unwrap();
        let Value::BytecodeFunction { chunk, .. } = VM::new().execute(script).unwrap() else {
            panic!("expected a bytecode function")
        };
        let info = jit.compile_function_with_info(&chunk, "read").unwrap();
        assert_eq!(info.param_count, arity);
        assert_eq!(jit.get_fn_info("read").unwrap().param_count, arity);
    }
}
