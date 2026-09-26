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

#[test]
fn captured_scalar_collection_struct_enum_and_callable_values_survive_return() {
    characterize(
        r#"
        struct Record { value: int }
        enum Outcome { Good, Bad }
        func factory() {
            let integer := 42
            let fraction := 1.5
            let flag := true
            let text := "captured"
            let array := [1, 2]
            let map := {"key": 3}
            let record := Record { value: 4 }
            let tag := Outcome::Good(5)
            let expected_tag := Outcome::Good(5)
            let callable := func(value) { return value + 1 }
            return func() {
                return integer == 42 && fraction == 1.5 && flag && text == "captured"
                    && array[1] == 2 && map["key"] == 3 && record.value == 4
                    && tag == expected_tag && callable(5) == 6
            }
        }
        let reader := factory()
        audit_ok := reader()
    "#,
    );
}

#[test]
fn captured_array_and_dictionary_mutations_persist_across_calls() {
    characterize(
        r#"
        func factory() {
            mut array := [0]
            mut map := {"count": 0}
            return func() {
                array[0] = array[0] + 1
                map["count"] = map["count"] + 2
                return array[0] + map["count"]
            }
        }
        let update_capture := factory()
        let first := update_capture()
        let second := update_capture()
        audit_ok := first == 3 && second == 6
    "#,
    );
}

#[test]
fn conditional_capture_survives_early_return_and_block_exit() {
    characterize(
        r#"
        func factory(enabled) {
            if enabled {
                let captured := 41
                return func() { return captured }
            }
            return func() { return 0 }
        }
        let yes := factory(true)
        let no := factory(false)
        audit_ok := yes() == 41 && no() == 0
    "#,
    );
}

#[test]
fn v1_while_and_loop_captures_keep_iteration_snapshots() {
    for loop_header in ["while index < 3", "loop"] {
        characterize(&format!(
            r#"
            func factory() {{
                mut callbacks := []
                mut index := 0
                {loop_header} {{
                    let captured := index
                    callbacks = push(callbacks, func() {{ return captured }})
                    index += 1
                    if index == 3 {{ break }}
                }}
                return callbacks
            }}
            let readers := factory()
            audit_ok := readers[0]() == 0 && readers[1]() == 1 && readers[2]() == 2
        "#,
        ));
    }
}

#[test]
fn deterministic_nested_capture_programs_retain_grandparent_parameters() {
    // Bounded generated programs exercise transitive capture without requiring
    // intermediate functions to reference the parameter themselves.
    for depth in 1..=12 {
        let mut body = "return seed".to_string();
        for _ in 0..depth {
            body = format!("return func() {{ {body} }}");
        }
        let calls = "reader = reader()\n".repeat(depth);
        characterize(&format!(
            "func factory(seed) {{ {body} }}\nmut reader := factory(41)\n{calls}audit_ok := reader == 41"
        ));
    }
}

#[test]
fn captured_let_and_const_reject_scalar_and_collection_writes() {
    for binding in ["let", "const"] {
        for (initial, write) in [
            ("0", "captured = 1"),
            ("[0]", "captured[0] = 1"),
            ("{\"value\": 0}", "captured[\"value\"] = 1"),
        ] {
            let source = format!(
                "func factory() {{ {binding} captured := {initial}; return func() {{ {write} }} }}\nlet callback := factory()\ncallback()"
            );
            let mut parser = Parser::new(tokenize(&source).unwrap());
            let parsed = parser.parse_with_diagnostics();
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            let mut interpreter = Interpreter::new();
            interpreter.eval_stmts(&parsed.stmts);
            let Some(Value::Error(interpreter_error)) = interpreter.return_value else {
                panic!("expected immutable capture error for {source}");
            };
            let mut vm = VM::new();
            vm.set_globals(Arc::new(Mutex::new(Interpreter::new().env)));
            let chunk = Compiler::new().compile(&parsed.stmts).unwrap();
            let vm_error = vm.execute(chunk).expect_err("immutable capture must be denied");
            let suffix = if binding == "let" {
                "immutable let binding: captured"
            } else {
                "const binding: captured"
            };
            // Existing assignment and in-place mutation diagnostics use different
            // verbs. Both engines must preserve the binding kind and name.
            assert!(interpreter_error.contains(suffix), "{interpreter_error}");
            assert!(vm_error.contains(suffix), "{vm_error}");
        }
    }
}

#[test]
#[ignore = "known interpreter defect: a named closure snapshot omits its own binding"]
fn a_recursive_named_closure_keeps_its_capture_after_parent_return() {
    characterize(
        r#"
        func factory(offset) {
            func fold_capture(n) {
                if n == 0 { return offset }
                return n + fold_capture(n - 1)
            }
            return fold_capture
        }
        let recursive := factory(7)
        audit_ok := recursive(3) == 13
    "#,
    );
}

#[test]
fn a_capture_escaped_before_throw_survives_defining_frame_unwind() {
    characterize(
        r#"
        mut escaped := null
        func factory() {
            let captured := 7
            escaped = func() { return captured }
            throw("factory failed after publishing closure")
        }
        mut caught := false
        try { factory() } except error { caught = true }
        audit_ok := caught && escaped() == 7
    "#,
    );
}

#[test]
fn closures_capture_runtime_created_bindings_and_legacy_receiver_fields() {
    for source in [
        r#"
        func factory() { value := 7; return func() { return value } }
        let read := factory()
        audit_ok := read() == 7
        "#,
        r#"
        func factory() { let [value, other] := [7, 8]; return func() { return value } }
        let read := factory()
        audit_ok := read() == 7
        "#,
        r#"
        func factory() {
            try { throw("message") }
            except error { return func() { return error.message } }
        }
        let read := factory()
        audit_ok := read() == "message"
        "#,
        r#"
        struct Test {
            x: float,
            func reader() { return func() { return x } }
        }
        let object := Test { x: 7.0 }
        let read := object.reader()
        audit_ok := read() == 7.0
        "#,
    ] {
        characterize(source);
    }
}

#[test]
fn for_iteration_binding_shadows_an_already_captured_name() {
    characterize(
        r#"
        func factory() {
            let value := 9
            return func() {
                let before := value
                mut readers := []
                for value in [1, 2] { readers = push(readers, func() { return value }) }
                return before == 9 && readers[0]() == 1 && readers[1]() == 2
            }
        }
        let check := factory()
        audit_ok := check()
    "#,
    );
}

#[test]
fn captures_survive_script_block_exit_and_root_loop_shadowing() {
    characterize(
        r#"
        mut escaped := null
        let value := 99
        if true {
            let value := 7
            escaped = func() { return value }
        }
        mut readers := []
        for value in [1, 2] { readers = push(readers, func() { return value }) }
        audit_ok := escaped() == 7 && value == 99 && readers[0]() == 1 && readers[1]() == 2
    "#,
    );
}

#[test]
fn loop_iterable_resolves_before_the_iteration_binding() {
    characterize(
        r#"
        func factory() {
            let values := [1, 2]
            mut readers := []
            for values in values { readers = push(readers, func() { return values }) }
            return readers
        }
        let readers := factory()
        audit_ok := readers[0]() == 1 && readers[1]() == 2
    "#,
    );
}

#[test]
fn script_block_captures_include_destructuring_and_new_bare_bindings() {
    characterize(
        r#"
        mut first := null
        mut second := null
        if true {
            let [value] := [7]
            first = func() { return value }
            created := 8
            second = func() { created += 1; return created }
        }
        audit_ok := first() == 7 && second() == 9 && second() == 10
    "#,
    );
}

#[test]
fn redefining_a_named_function_preserves_earlier_function_values() {
    characterize(
        r#"
        func factory() {
            func read() { return 1 }
            let before := read
            func read() { return 2 }
            return [before, read]
        }
        let readers := factory()
        audit_ok := readers[0]() == 1 && readers[1]() == 2
    "#,
    );
}
