//! Phase-B generator contracts. Finite fixtures also exercise caller restoration.
use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};

fn execute(vm: &mut VM, source: &str) -> Result<Value, String> {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    vm.execute(Compiler::new().compile(&parsed.stmts).unwrap())
}

fn setup(source: &str) -> (VM, Arc<Mutex<Environment>>, Value) {
    let env = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(env.clone());
    execute(&mut vm, source).unwrap();
    let generator = env.lock().unwrap().get("instance").unwrap();
    (vm, env, generator)
}

fn step(vm: &mut VM, generator: &Value) -> Option<Value> {
    match vm.generator_next(generator.clone()).unwrap() {
        Value::Option { is_some: true, value } => Some(*value),
        Value::Option { is_some: false, .. } => None,
        other => panic!("unexpected generator step: {other:?}"),
    }
}

fn number(value: Option<Value>) -> i64 {
    match value {
        Some(Value::Int(n)) => n,
        other => panic!("{other:?}"),
    }
}

#[test]
fn vm_generator_runs_nested_calls_and_all_loop_iterations() {
    let (mut vm, env, generator) = setup("func twice(n) { return n * 2 } func* values() { mut n := 1 while n < 4 { yield twice(n) n += 1 } yield 9 } let instance := values()");
    for expected in [2, 4, 6, 9] {
        assert_eq!(number(step(&mut vm, &generator)), expected);
        assert_eq!(env.lock().unwrap().scopes.len(), 1);
    }
    assert!(step(&mut vm, &generator).is_none());
    assert!(step(&mut vm, &generator.clone()).is_none());
}

#[test]
fn vm_generator_preserves_handlers_across_yields() {
    let (mut vm, env, generator) = setup("func* values() { try { yield 1 throw(\"resume failure\") } except err { yield 2 } yield 3 } let instance := values()");
    for expected in [1, 2, 3] {
        assert_eq!(number(step(&mut vm, &generator)), expected);
        assert_eq!(env.lock().unwrap().scopes.len(), 1);
    }
    assert!(step(&mut vm, &generator).is_none());
}

#[test]
fn vm_generator_caches_failure_and_restores_caller() {
    let (mut vm, env, generator) =
        setup("func* values() { yield 1 missing } let instance := values()");
    assert_eq!(number(step(&mut vm, &generator)), 1);
    let first = vm.generator_next(generator.clone()).unwrap_err();
    assert!(first.contains("Undefined variable"), "{first}");
    assert_eq!(vm.generator_next(generator).unwrap_err(), first);
    assert_eq!(env.lock().unwrap().scopes.len(), 1);
    execute(&mut vm, "let recovered := 42").unwrap();
    assert_eq!(number(env.lock().unwrap().get("recovered")), 42);
}

#[test]
fn vm_for_break_consumes_only_one_generator_item() {
    let (mut vm, env, _) = setup("mut produced := 0 func* values() { produced += 1 yield 1 produced += 1 yield 2 } let instance := values() for item in instance { break }");
    assert_eq!(number(env.lock().unwrap().get("produced")), 1);
    execute(&mut vm, "mut rest := 0 for item in instance { rest += item }").unwrap();
    assert_eq!(number(env.lock().unwrap().get("rest")), 2);
}

#[test]
fn vm_generator_yields_null_and_return_only_completes() {
    let (mut vm, _, generator) =
        setup("func* values() { yield null return 99 } let instance := values()");
    assert!(matches!(step(&mut vm, &generator), Some(Value::Null)));
    assert!(step(&mut vm, &generator).is_none());
}

#[test]
fn vm_generator_reentry_is_an_error_without_deadlock() {
    let (mut vm, _, generator) =
        setup("func* values() { for item in instance { yield item } } let instance := values()");
    let error = vm.generator_next(generator.clone()).unwrap_err();
    assert!(error.contains("already being resumed"), "{error}");
    assert_eq!(vm.generator_next(generator).unwrap_err(), error);
}

#[test]
fn vm_generator_does_not_resolve_resumer_local_bindings() {
    let (mut vm, env, _) = setup("func* values() { yield hidden } let instance := values()");
    execute(&mut vm, "let caught := false func consume() { let hidden := 9 try { for item in instance { print(item) } } except err { return true } return false } let recovered := consume()").unwrap();
    assert!(matches!(env.lock().unwrap().get("recovered"), Some(Value::Bool(true))));
}

#[test]
fn vm_generator_never_increases_creator_or_resumer_authority() {
    use kujo::interpreter::RuntimeCapabilityPolicy;
    for restrict_creator in [false, true] {
        let env = Arc::new(Mutex::new(Interpreter::new().env));
        let mut vm = VM::new();
        vm.set_globals(env.clone());
        if restrict_creator {
            vm.set_capability_policy(RuntimeCapabilityPolicy::restricted());
        }
        execute(
            &mut vm,
            "func* values() { yield read_file(\"must-not-be-opened\") } let instance := values()",
        )
        .unwrap();
        let generator = env.lock().unwrap().get("instance").unwrap();
        vm.set_capability_policy(if restrict_creator {
            RuntimeCapabilityPolicy::trusted()
        } else {
            RuntimeCapabilityPolicy::restricted()
        });
        let error = vm.generator_next(generator).unwrap_err();
        assert!(error.contains("--allow-fs-read"), "{error}");
    }
}

#[test]
fn vm_generator_rejects_invalid_local_slot_and_restores_caller() {
    use kujo::bytecode::{Constant, OpCode};
    let mut parser =
        Parser::new(tokenize("func* values() { yield 1 } let instance := values()").unwrap());
    let mut chunk = Compiler::new().compile(&parser.parse_with_diagnostics().stmts).unwrap();
    for constant in &mut chunk.constants {
        if let Constant::Function(function) = constant {
            function.instructions = vec![OpCode::LoadLocal(usize::MAX), OpCode::Yield];
        }
    }
    let env = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(env.clone());
    vm.execute(chunk).unwrap();
    let generator = env.lock().unwrap().get("instance").unwrap();
    assert!(vm.generator_next(generator).unwrap_err().contains("local slot"));
    execute(&mut vm, "let recovered := 42").unwrap();
    assert_eq!(number(env.lock().unwrap().get("recovered")), 42);
}

#[test]
fn vm_generator_has_no_automatic_global_environment_ownership_cycle() {
    let (vm, env, generator) = setup("func* values() { yield 1 } let instance := values()");
    let environment = Arc::downgrade(&env);
    drop(vm);
    drop(env);
    assert!(environment.upgrade().is_none());
    drop(generator);
}

fn interpret(interpreter: &mut Interpreter, source: &str) {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    interpreter.eval_stmts(&parsed.stmts);
}

#[test]
fn interpreter_generator_preserves_nested_control_flow_and_shared_progress() {
    let mut interpreter = Interpreter::new();
    interpret(&mut interpreter, "func twice(n) { return n * 2 } func* values() { mut n := 0 while n < 4 { n += 1 if n == 2 { continue } yield twice(n) } for n in [1, 2, 3] { if n == 3 { break } yield n } try { yield 10 missing } except err { yield 20 } return 99 } let instance := values() let alias := instance mut total := 0 for n in instance { total += n break } for n in alias { total += n } for n in instance { total += 1000 }");
    assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    assert_eq!(number(interpreter.env.get("total")), 49);
    assert_eq!(interpreter.env.scopes.len(), 1);
}

#[test]
fn interpreter_generator_caches_errors_and_restores_caller() {
    let mut interpreter = Interpreter::new();
    interpret(&mut interpreter, "func* values() { yield 1 missing } let instance := values() mut caught := 0 try { for n in instance { print(n) } } except err { caught += 1 } try { for n in instance { print(n) } } except err { caught += 1 } let recovered := 42");
    assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    assert_eq!(number(interpreter.env.get("caught")), 2);
    assert_eq!(number(interpreter.env.get("recovered")), 42);
    assert_eq!(interpreter.env.scopes.len(), 1);
}

#[test]
fn interpreter_generator_retains_factory_capture_without_dynamic_resumer_scope() {
    let mut interpreter = Interpreter::new();
    interpret(&mut interpreter, "func factory(start) { func* values() { mut n := start yield n n += 1 yield n } return values() } let a := factory(2) let b := factory(10) func consume(g) { let start := 99 mut total := 0 for n in g { total += n } return total } let total := consume(a) + consume(b)");
    assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    assert_eq!(number(interpreter.env.get("total")), 26);
    assert_eq!(interpreter.env.scopes.len(), 1);
}

#[test]
fn interpreter_generator_never_increases_creator_or_resumer_authority() {
    use kujo::interpreter::RuntimeCapabilityPolicy;
    for restrict_creator in [false, true] {
        let mut interpreter = Interpreter::new();
        interpreter.set_capability_policy(if restrict_creator {
            RuntimeCapabilityPolicy::restricted()
        } else {
            RuntimeCapabilityPolicy::trusted()
        });
        interpret(
            &mut interpreter,
            "func* values() { yield read_file(\"must-not-be-opened\") } let instance := values()",
        );
        assert!(interpreter.return_value.is_none());
        interpreter.set_capability_policy(if restrict_creator {
            RuntimeCapabilityPolicy::trusted()
        } else {
            RuntimeCapabilityPolicy::restricted()
        });
        interpret(&mut interpreter, "for n in instance { print(n) }");
        assert!(format!("{:?}", interpreter.return_value).contains("--allow-fs-read"));
        assert_eq!(interpreter.env.scopes.len(), 1);
    }
}

#[test]
fn vm_callback_for_loop_uses_full_dispatch() {
    let (mut vm, env, _) = setup("func* values() { yield 1 } let instance := values()");
    execute(&mut vm, "func total(n) { mut sum := 0 for item in [n, n + 1] { sum += item } return sum } let result := map([1, 2], total)").unwrap();
    assert!(
        matches!(env.lock().unwrap().get("result"), Some(Value::Array(values)) if matches!(values.as_slice(), [Value::Int(3), Value::Int(5)]))
    );
}

#[test]
fn vm_captured_mutability_errors_are_catchable_in_the_owning_frame() {
    let (mut vm, env, _) = setup("func* values() { yield 1 } let instance := values()");
    execute(&mut vm, "func factory() { let immutable := 7 return func() { try { immutable = 9 } except err { return immutable } return 0 } } let result := factory()()").unwrap();
    assert_eq!(number(env.lock().unwrap().get("result")), 7);
}

#[test]
fn async_generators_reject_instead_of_selecting_an_ambiguous_call_kind() {
    let mut parser = Parser::new(tokenize("async func* unsupported() { yield 1 }").unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(Compiler::new()
        .compile(&parsed.stmts)
        .unwrap_err()
        .contains("Async generators are not supported"));
    let mut interpreter = Interpreter::new();
    interpreter.eval_stmts(&parsed.stmts);
    assert!(
        matches!(interpreter.return_value, Some(Value::Error(ref e)) if e.contains("Async generators are not supported"))
    );
}

#[test]
fn interpreter_generator_closure_does_not_retain_unreferenced_generator_root() {
    for body in [
        "func local() { return 1 } yield local()",
        "mut n := 1 func local() { n += 1 return n } yield local()",
    ] {
        let source = format!(
            "func* numbers() {{ {body} }} let instance := numbers() for n in instance {{ break }}"
        );
        let mut parser = Parser::new(tokenize(&source).unwrap());
        let parsed = parser.parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        let mut interpreter = Interpreter::new();
        interpreter.eval_stmts(&parsed.stmts);
        assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
        let weak = match interpreter.env.get("instance").unwrap() {
            Value::Generator { state, .. } => Arc::downgrade(&state),
            other => panic!("{other:?}"),
        };
        drop(interpreter);
        assert_eq!(weak.strong_count(), 0, "unreferenced root retained: {body}");
    }
}
