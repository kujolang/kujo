// Symbolized diagnostic harness; link against the recorded accepted Kujo rlib.
// No allocator wrapper, capture instrumentation or elapsed-speed comparison.
use kujo::{
    compiler::Compiler,
    interpreter::{Environment, Value},
    lexer,
    parser::Parser,
    vm::{VmExecutionResult, VM},
};
use std::sync::{Arc, Mutex};
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(arguments.len(), 2, "vm|jit CASE");
    assert!(["vm", "jit"].contains(&arguments[0].as_str()));
    assert!(!arguments[1].contains(['"', '\\', '\n', '\r']));
    let port = std::fs::read_to_string("benchmarks/langarena/port.kujo").unwrap();
    assert_eq!(port.matches("let arguments := args()").count(), 1);
    let binding = format!(
        "let arguments := [\"benchmarks/langarena/test.json\", \"{}\", \"--profile\"]",
        arguments[1]
    );
    let source = port.replacen("let arguments := args()", &binding, 1);
    let jit = arguments[0] == "jit";
    std::thread::Builder::new()
        .name("kujo-kernel-profile".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
            let statements = parser.parse();
            let chunk = Compiler::new().compile(&statements).unwrap();
            let mut vm = VM::new();
            vm.set_jit_enabled(jit);
            let globals = Arc::new(Mutex::new(Environment::new()));
            {
                let mut env = globals.lock().unwrap();
                for (name, value) in kujo::builtins::get_builtins() {
                    env.set(name, value);
                }
                env.set("null".into(), Value::Null);
            }
            vm.set_globals(globals);
            match vm.execute_until_suspend(chunk).unwrap() {
                VmExecutionResult::Completed => {}
                VmExecutionResult::Suspended { .. } => {
                    vm.run_scheduler_until_complete_unbounded().unwrap()
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
