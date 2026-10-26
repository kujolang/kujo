//! Embedded AST task bodies and VM callbacks share task globals, not parent globals.
use kujo::{
    bytecode::Constant,
    compiler::Compiler,
    interpreter::{Interpreter, Value},
    lexer::tokenize,
    parser::Parser,
    vm::VM,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
fn parse(source: &str) -> Vec<kujo::ast::Stmt> {
    Parser::new(tokenize(source).unwrap()).parse()
}
#[test]
fn embedded_task_callbacks_preserve_current_globals_across_calls_and_errors() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("embedded_helper.kujo"),
        "export func call_work(work) { return work(false) }\n",
    )
    .unwrap();
    let mut failures = Vec::new();
    for (case, relay, expected) in [
        ("read", "func relay(work) { return global_value }", vec![9, 9, 1]),
        ("write", "func relay(work) { global_value = 10 return global_value }", vec![10, 10, 1]),
        (
            "caught_error",
            "func relay(work) { global_value = 10 throw(\"expected\") }",
            vec![10, 10, 1],
        ),
        (
            "foreign_reentry",
            "func relay(work) { global_value = 10 return call_work(work) }",
            vec![10, 10, 1],
        ),
    ] {
        for mode in ["interpreter", "vm", "jit"] {
            let compiled = Compiler::new().compile(&parse(relay)).unwrap();
            let function = compiled
                .constants
                .iter()
                .find_map(|value| match value {
                    Constant::Function(chunk) => Some((**chunk).clone()),
                    _ => None,
                })
                .unwrap();
            let mut interpreter = Interpreter::new();
            interpreter.module_loader.add_search_path(directory.path());
            interpreter.env.define(
                "relay".into(),
                Value::BytecodeFunction {
                    chunk: function,
                    captured: HashMap::new(),
                    captured_binding_kinds: HashMap::new(),
                },
            );
            interpreter.eval_stmts(&parse(
                r#"
from embedded_helper import call_work
mut global_value := 0
func factory() {
 mut count := 0
 func work(write) {
  if !write { return global_value }
  global_value = 9
  count += 1
  mut result := -1
  try { result = relay(work) } except error { result = global_value }
  return [result,global_value,count]
 }
 return work
}
let work := factory()
"#,
            ));
            let source =
                parse("let actual := await await_task(spawn_task(func() { return work(true) }))");
            let (actual, parent) = if mode == "interpreter" {
                interpreter.eval_stmts(&source);
                (interpreter.env.get("actual"), interpreter.env.get("global_value"))
            } else {
                let globals = Arc::new(Mutex::new(interpreter.env.clone()));
                let mut vm = VM::new();
                vm.set_jit_enabled(mode == "jit");
                vm.set_globals(globals.clone());
                let result =
                    vm.execute_until_suspend(Compiler::new().compile(&source).unwrap()).and_then(
                        |_| vm.run_scheduler_until_complete_with_timeout(Duration::from_secs(30)),
                    );
                if let Err(error) = result {
                    failures.push(format!("{case}/{mode}: {error}"));
                    continue;
                }
                let globals = globals.lock().unwrap();
                (globals.get("actual"), globals.get("global_value"))
            };
            let ints = match &actual {
                Some(Value::Array(values)) => values
                    .iter()
                    .map(|value| if let Value::Int(n) = value { Some(*n) } else { None })
                    .collect::<Option<Vec<_>>>(),
                _ => None,
            };
            if ints.as_ref() != Some(&expected) || !matches!(parent, Some(Value::Int(0))) {
                failures.push(format!("{case}/{mode}: values={ints:?}, actual={actual:?}, parent={parent:?}, expected={expected:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
