//! Phase-A observations, not desired Phase-B contracts. See the completion plan.
use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::{VmExecutionResult, VM};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn program(source: &str) -> Vec<kujo::ast::Stmt> {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let result = parser.parse_with_diagnostics();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(!result.stmts.is_empty());
    result.stmts
}

fn interpreter(source: &str) -> Interpreter {
    let mut interpreter = Interpreter::new();
    interpreter.eval_stmts(&program(source));
    interpreter
}

fn vm(source: &str) -> (Result<(), String>, Arc<Mutex<Environment>>) {
    let env = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(env.clone());
    let result = Compiler::new().compile(&program(source)).and_then(|chunk| {
        match vm.execute_until_suspend(chunk)? {
            VmExecutionResult::Completed => Ok(()),
            VmExecutionResult::Suspended { .. } => {
                vm.run_scheduler_until_complete_with_timeout(Duration::from_secs(5))
            }
        }
    });
    (result, env)
}

fn integer(value: Option<Value>) -> i64 {
    match value {
        Some(Value::Int(n)) => n,
        other => panic!("expected integer, got {other:?}"),
    }
}

fn sums(body: &str, expected_interpreter: i64, expected_vm: i64) {
    let source =
        format!("func* values() {{ {body} }} mut observed_total := 0 for n in values() {{ observed_total += n }}");
    let interp = interpreter(&source);
    assert!(interp.return_value.is_none(), "{:?}", interp.return_value);
    assert_eq!(integer(interp.env.get("observed_total")), expected_interpreter);
    let (result, env) = vm(&source);
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("observed_total")), expected_vm);
}

#[test]
fn straight_line_generator_state_survives_yields() {
    sums("mut x := 2 yield x x += 3 yield x", 7, 7);
}

#[test]
fn loop_generator_vm_resumes_all_values_interpreter_pending() {
    sums("mut x := 1 while x < 4 { yield x x += 1 }", 1, 6);
}

#[test]
fn explicit_return_is_currently_a_yield_only_in_interpreter() {
    sums("yield 1 return 9", 10, 1);
}

#[test]
fn generator_alias_iteration_restarts_only_in_interpreter() {
    let source = "func* values() { yield 1 yield 2 } let a := values() let b := a mut observed_total := 0 for n in a { observed_total += n } for n in b { observed_total += n }";
    assert_eq!(integer(interpreter(source).env.get("observed_total")), 6);
    let (result, env) = vm(source);
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("observed_total")), 3);
}

#[test]
fn nested_call_in_generator_uses_normal_vm_dispatch() {
    let source = "func answer() { return 42 } func* values() { yield answer() } mut observed_total := 0 for n in values() { observed_total += n }";
    assert_eq!(integer(interpreter(source).env.get("observed_total")), 42);
    let (result, env) = vm(source);
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("observed_total")), 42);
}

#[test]
fn generator_error_is_currently_swallowed_only_by_interpreter() {
    let source =
        "func* values() { yield 1 yield missing } mut observed_total := 0 for n in values() { observed_total += n }";
    let interp = interpreter(source);
    // Inspect the error as a standalone statement, rather than yielding an error value.
    assert!(interp.return_value.is_some());
    assert!(vm(source).0.is_err());
    let source = "func* values() { yield 1 missing } mut observed_total := 0 for n in values() { observed_total += n }";
    let interp = interpreter(source);
    assert!(interp.return_value.is_none(), "{:?}", interp.return_value);
    assert_eq!(integer(interp.env.get("observed_total")), 1);
    assert!(vm(source).0.is_err());
}

#[test]
fn promise_reawait_and_await_value_are_supported() {
    let source = "async func value() { return 21 } let p := value() let a := await p let b := await p let observed_total := a + b + await 1";
    assert_eq!(integer(interpreter(source).env.get("observed_total")), 43);
    let (result, env) = vm(source);
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("observed_total")), 43);
}

#[test]
fn vm_spawn_body_is_discarded() {
    let (result, env) =
        vm("mut marker := 0 spawn { marker = 99 missing } let observed_total := marker");
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("observed_total")), 0);
}

#[test]
fn spawn_task_interpreter_returns_placeholder_without_executing_body() {
    let interp = interpreter("async func work() { return 42 } let handle := spawn_task(work) let result := await await_task(handle)");
    assert!(interp.return_value.is_none(), "{:?}", interp.return_value);
    assert!(matches!(interp.env.get("result"), Some(Value::Null)));
    let result = vm("async func work() { return 42 } let handle := spawn_task(work)").0;
    assert!(result.unwrap_err().contains("requires an async function"));
}

#[test]
fn later_loop_backedge_does_not_exhaust_an_earlier_yield() {
    sums("yield 1 while false { yield 9 } yield 2", 3, 3);
}

#[test]
fn nested_if_continuation_is_skipped_only_by_interpreter() {
    sums("if true { yield 1 yield 2 } yield 3", 4, 6);
}

#[test]
fn vm_for_does_not_drain_generator_before_consumer_break() {
    let source = "mut produced := 0 func* values() { produced += 1 yield 1 produced += 1 yield 2 } for n in values() { break }";
    let interp = interpreter(source);
    // Interpreter generator environment is a snapshot, so parent is unchanged.
    assert!(interp.return_value.is_none(), "{:?}", interp.return_value);
    assert_eq!(integer(interp.env.get("produced")), 0);
    let (result, env) = vm(source);
    result.unwrap();
    assert_eq!(integer(env.lock().unwrap().get("produced")), 1);
}

#[test]
fn async_body_error_occurs_at_call_only_in_vm() {
    let source = "async func fail() { missing } let pending := fail() let continued := 1";
    let interp = interpreter(source);
    assert!(interp.return_value.is_none(), "{:?}", interp.return_value);
    assert!(matches!(interp.env.get("pending"), Some(Value::Promise { .. })));
    assert_eq!(integer(interp.env.get("continued")), 1);
    let (result, env) = vm(source);
    assert!(result.unwrap_err().contains("Undefined variable"));
    assert!(env.lock().unwrap().get("continued").is_none());
}
