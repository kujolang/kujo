use kujo::compiler::Compiler;
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::{VmExecutionResult, VM};
use std::sync::{Arc, Mutex};

fn assert_both(source: &str) {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut interpreter = Interpreter::new();
    interpreter.eval_stmts(&parsed.stmts);
    assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    assert!(
        matches!(interpreter.env.get("ok"), Some(Value::Bool(true))),
        "interpreter result: {:?}",
        interpreter.env.get("ok")
    );
    let mut vm = VM::new();
    let environment = Arc::new(Mutex::new(Interpreter::new().env));
    vm.set_globals(environment.clone());
    if matches!(
        vm.execute_until_suspend(Compiler::new().compile(&parsed.stmts).unwrap()).unwrap(),
        VmExecutionResult::Suspended { .. }
    ) {
        vm.run_scheduler_until_complete_with_timeout(std::time::Duration::from_secs(5)).unwrap();
    }
    assert!(
        matches!(environment.lock().unwrap().get("ok"), Some(Value::Bool(true))),
        "VM result: {:?}",
        environment.lock().unwrap().get("ok")
    );
}

#[test]
fn async_body_failure_is_reported_when_awaited() {
    assert_both("async func fail() { missing } let p := fail() let continued := true mut caught := false try { await p } except err { caught = true } let ok := continued && caught");
}

#[test]
fn tasks_execute_real_bodies_and_repeat_waits_share_results() {
    assert_both("func work() { return 21 } let h := spawn_task(work) let a := await await_task(h) let b := await await_task(h) let ok := a + b == 42");
}

#[test]
fn async_captures_preserve_mutation_across_completed_calls() {
    assert_both("func factory() { mut n := 0 async func counter() { n += 1 return n } return counter } let c := factory() let a := await c() let b := await c() let ok := a == 1 && b == 2");
}

#[test]
fn nested_async_calls_use_independent_execution_owners() {
    assert_both("async func leaf() { return 20 } async func outer() { let n := await leaf() return n + 1 } let a := outer() let b := outer() let x := await a let y := await b let ok := x + y == 42");
}

#[test]
fn task_arity_is_checked_before_submission() {
    assert_both("func work(value) { return value } mut caught := false try { spawn_task(work) } except err { caught = true } let ok := caught");
}

#[test]
fn completed_task_cancellation_does_not_replace_its_result() {
    assert_both("func work() { return 42 } let h := spawn_task(work) let a := await await_task(h) let cancelled := cancel_task(h) let b := await await_task(h) let ok := cancelled == false && a == 42 && b == 42");
}

#[test]
fn anonymous_async_functions_return_promises() {
    assert_both("let f := async func() { return 42 } let p := f() let is_promise := type(p) == \"promise\" let value := await p let ok := is_promise && value == 42");
}
