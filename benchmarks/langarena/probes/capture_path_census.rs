// Link only against the isolated instrument-capture-census.rb library.
// This probe measures path reachability, never speed or allocation cost.
use kujo::{
    compiler::Compiler,
    interpreter::{Interpreter, Value},
    lexer,
    parser::Parser,
    vm::VM,
};
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(arguments.len(), 2, "MODE CASE|FIXTURE_PATH");
    let mode = arguments[0].clone();
    assert!(["vm", "interpreter", "jit"].contains(&mode.as_str()));
    let source = if std::path::Path::new(&arguments[1]).is_file() {
        std::fs::read_to_string(&arguments[1]).unwrap()
    } else {
        let port = std::fs::read_to_string("benchmarks/langarena/port.kujo").unwrap();
        assert_eq!(port.matches("let arguments := args()").count(), 1);
        let injected =
            format!("let arguments := [\"benchmarks/langarena/test.json\", \"{}\"]", arguments[1]);
        port.replacen("let arguments := args()", &injected, 1)
    };
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
            let stmts = parser.parse();
            if mode == "interpreter" {
                let mut interpreter = Interpreter::new();
                interpreter.eval_stmts(&stmts);
                assert!(
                    !matches!(
                        interpreter.return_value,
                        Some(Value::Error(_)) | Some(Value::ErrorObject { .. })
                    ),
                    "interpreter runtime failure"
                );
            } else {
                let chunk = Compiler::new().compile(&stmts).unwrap();
                let mut vm = VM::new();
                vm.set_jit_enabled(mode == "jit");
                let globals = std::sync::Arc::new(std::sync::Mutex::new(
                    kujo::interpreter::Environment::new(),
                ));
                {
                    let mut env = globals.lock().unwrap();
                    for (name, value) in kujo::builtins::get_builtins() {
                        env.set(name, value);
                    }
                    env.set("null".into(), Value::Null);
                }
                vm.set_globals(globals);
                // Match `kujo run --scheduler-no-timeout`, including threaded
                // Matmul and any other benchmark that suspends for task results.
                match vm.execute_until_suspend(chunk).unwrap() {
                    kujo::vm::VmExecutionResult::Completed => {}
                    kujo::vm::VmExecutionResult::Suspended { .. } => {
                        vm.run_scheduler_until_complete_unbounded().unwrap();
                    }
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!("CAPTURE_CENSUS {:?}", kujo::capture_census::snapshot());
}
