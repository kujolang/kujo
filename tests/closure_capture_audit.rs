//! Characterization of v1 capture identity, not a specification of future upvalues.
//! See docs/CLOSURE_UPVALUE_AUDIT.md before changing these expectations.
use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};

fn characterize(source: &str) {
    let mut parser = Parser::new(tokenize(source).expect("valid tokens"));
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = parsed.stmts;
    let mut interpreter = Interpreter::new();
    interpreter.eval_stmts(&program);
    assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    assert!(matches!(interpreter.env.get("audit_ok"), Some(Value::Bool(true))));

    let globals: Arc<Mutex<Environment>> = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    let chunk = Compiler::new().compile(&program).expect("audit compiles");
    vm.execute(chunk).expect("audit executes");
    assert!(matches!(globals.lock().unwrap().get("audit_ok"), Some(Value::Bool(true))));
}

#[test]
fn v1_separately_created_siblings_have_independent_capture_snapshots() {
    characterize(
        r#"
        func make_pair() {
            mut count := 0
            func increment() { count += 1; return count }
            func current() { return count }
            return [increment, current]
        }
        let pair := make_pair()
        let first := pair[0]()
        let observed := pair[1]()
        let second := pair[0]()
        let observed_again := pair[1]()
        audit_ok := first == 1 && observed == 0 && second == 2 && observed_again == 0
    "#,
    );
}

#[test]
fn v1_parent_assignment_does_not_update_an_existing_capture() {
    characterize(
        r#"
        func factory() {
            mut value := 1
            let before := func() { return value }
            value = 2
            let after := func() { return value }
            return [before, after]
        }
        let readers := factory()
        audit_ok := readers[0]() == 1 && readers[1]() == 2
    "#,
    );
}

#[test]
fn v1_aliases_share_a_closure_but_factory_calls_are_independent() {
    characterize(
        r#"
        func factory() {
            mut count := 0
            let increment := func() { count += 1; return count }
            return [increment, increment]
        }
        let left := factory()
        let right := factory()
        let a := left[0]()
        let b := left[1]()
        let c := right[0]()
        audit_ok := a == 1 && b == 2 && c == 1
    "#,
    );
}

#[test]
fn v1_transitive_captures_snapshot_the_intermediate_capture() {
    characterize(
        r#"
        func outer() {
            mut count := 0
            func middle() {
                count += 1
                return func() { count += 1; return count }
            }
            return middle
        }
        let maker := outer()
        let first := maker()
        let second := maker()
        let a := first()
        let b := second()
        let c := first()
        audit_ok := a == 2 && b == 3 && c == 3
    "#,
    );
}

#[test]
fn v1_capture_mutation_survives_a_caught_throw() {
    characterize(
        r#"
        func factory() {
            mut count := 0
            return func(fail) {
                count += 1
                if fail { throw("audit") }
                return count
            }
        }
        let callback := factory()
        mut caught := false
        try { callback(true) } except error { caught = true }
        audit_ok := caught && callback(false) == 2
    "#,
    );
}

#[test]
#[ignore = "known VM defect: MakeClosure chooses a later, inactive same-named slot"]
fn closure_must_capture_the_binding_visible_at_its_definition() {
    characterize(
        r#"
        func factory() {
            let value := 1
            let reader := func() { return value }
            if true { let value := 2 }
            return reader
        }
        let reader := factory()
        audit_ok := reader() == 1
    "#,
    );
}

#[test]
#[ignore = "known VM defect: nested FuncDef uses StoreGlobal"]
fn nested_function_must_not_define_a_global_binding() {
    characterize(
        r#"
        func outer() {
            func middle() { return 1 }
            return middle
        }
        let middle := outer()
        audit_ok := middle() == 1
    "#,
    );
}
