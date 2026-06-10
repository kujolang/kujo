use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::{VmExecutionResult, VM};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn run_interpreter(code: &str) -> Interpreter {
    let tokens = tokenize(code).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse();
    let mut interp = Interpreter::new();
    interp.eval_stmts(&program);
    interp
}

fn interpreter_error(code: &str) -> Option<String> {
    let interp = run_interpreter(code);
    match interp.return_value {
        Some(Value::Error(error)) => Some(error),
        Some(Value::Return(value)) => match *value {
            Value::Error(error) => Some(error),
            _ => None,
        },
        _ => None,
    }
}

fn run_vm(code: &str, env: Arc<Mutex<Environment>>) -> Result<(), String> {
    let tokens = tokenize(code).map_err(|diagnostics| {
        diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "unknown lexer error".to_string())
    })?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse();

    let mut compiler = Compiler::new();
    let chunk = compiler.compile(&program)?;

    let mut vm = VM::new();
    vm.set_globals(env);

    match vm.execute_until_suspend(chunk)? {
        VmExecutionResult::Completed => Ok(()),
        VmExecutionResult::Suspended { .. } => {
            vm.run_scheduler_until_complete_with_timeout(Duration::from_millis(5_000))
        }
    }
}

fn vm_env_with_builtins() -> Arc<Mutex<Environment>> {
    let interp = Interpreter::new();
    Arc::new(Mutex::new(interp.env))
}

fn assert_interpreter_and_vm_error_contains(script: &str, expected: &str) {
    let interp_error =
        interpreter_error(script).expect("expected interpreter execution to report an error");
    assert!(
        interp_error.contains(expected),
        "expected interpreter error containing {:?}, got {:?}",
        expected,
        interp_error
    );

    let vm_env = vm_env_with_builtins();
    let vm_error = run_vm(script, vm_env).expect_err("expected VM execution to report an error");
    assert!(
        vm_error.contains(expected),
        "expected VM error containing {:?}, got {:?}",
        expected,
        vm_error
    );
}

fn assert_interpreter_and_vm_env_bool(script: &str, binding_name: &str) {
    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "unexpected interpreter runtime result: {:?}",
        interp.return_value
    );
    assert!(
        matches!(interp.env.get(binding_name), Some(Value::Bool(true))),
        "expected interpreter binding {:?} to be true, got {:?}",
        binding_name,
        interp.env.get(binding_name)
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "unexpected vm runtime result: {:?}", vm_result);

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(
        matches!(vm_globals.get(binding_name), Some(Value::Bool(true))),
        "expected vm binding {:?} to be true, got {:?}",
        binding_name,
        vm_globals.get(binding_name)
    );
}

#[test]
fn spec_mut_binding_allows_reassignment_and_in_place_mutation() {
    let script = r#"
        mut total := 1
        total := total + 1

        mut series := [1, 2, 3]
        series[0] := total

        contract_ok := total == 2 && series[0] == 2
    "#;

    assert_interpreter_and_vm_env_bool(script, "contract_ok");
}

#[test]
fn spec_let_and_const_bindings_reject_reassignment_and_in_place_mutation() {
    let let_reassign = r#"
        let x := 1
        x := 2
    "#;
    assert_interpreter_and_vm_error_contains(let_reassign, "immutable");

    let let_in_place_mutation = r#"
        let bag := [1]
        bag[0] := 2
    "#;
    assert_interpreter_and_vm_error_contains(let_in_place_mutation, "immutable");

    let const_reassign = r#"
        const x := 1
        x := 2
    "#;
    assert_interpreter_and_vm_error_contains(const_reassign, "Cannot reassign const binding");
}

#[test]
fn spec_scope_rules_cover_shadowing_duplicate_declarations_and_loop_binding_lifetime() {
    let shadowing_script = r#"
        func check_shadowing() {
            let outer := 10
            seen_inner := 0

            if true {
                let outer := 20
                seen_inner := outer
            }

            return seen_inner == 20 && outer == 10
        }

        contract_ok := check_shadowing()
    "#;
    assert_interpreter_and_vm_env_bool(shadowing_script, "contract_ok");

    let duplicate_declaration = r#"
        let x := 1
        let x := 2
    "#;
    assert_interpreter_and_vm_error_contains(duplicate_declaration, "Duplicate declaration");

    let leaked_loop_binding = r#"
        func leak_test() {
            total := 0
            for item in [1, 2, 3] {
                total := total + item
            }
            return item
        }

        leak_test()
    "#;
    assert_interpreter_and_vm_error_contains(leaked_loop_binding, "Undefined variable: item");
}

#[test]
fn spec_function_fallthrough_and_bare_return_default_to_null() {
    let script = r#"
        func implicit_null() {
            let local := 1
        }

        func explicit_bare_return() {
            return
        }

        value_a := implicit_null()
        value_b := explicit_bare_return()
        contract_ok := value_a == null && value_b == null
    "#;

    assert_interpreter_and_vm_env_bool(script, "contract_ok");
}

#[test]
fn spec_callable_arity_contract_errors_include_expected_and_received_counts() {
    let too_few = r#"
        func add(a, b) {
            return a + b
        }
        return add(1)
    "#;
    assert_interpreter_and_vm_error_contains(too_few, "add expects 2 arguments, got 1");

    let too_many = r#"
        func add(a, b) {
            return a + b
        }
        return add(1, 2, 3)
    "#;
    assert_interpreter_and_vm_error_contains(too_many, "add expects 2 arguments, got 3");
}

#[test]
fn spec_truthiness_and_logical_operator_contracts_are_boolean_and_short_circuiting() {
    let script = r#"
        falsey_hits := 0
        if false { falsey_hits += 1 }
        if null { falsey_hits += 1 }
        if 0 { falsey_hits += 1 }
        if 0.0 { falsey_hits += 1 }
        if "" { falsey_hits += 1 }
        if [] { falsey_hits += 1 }
        if {} { falsey_hits += 1 }

        truthy_hits := 0
        if true { truthy_hits += 1 }
        if 1 { truthy_hits += 1 }
        if -1 { truthy_hits += 1 }
        if 0.5 { truthy_hits += 1 }
        if "false" { truthy_hits += 1 }
        if [0] { truthy_hits += 1 }
        if {"k": 1} { truthy_hits += 1 }

        and_value := true && 99
        or_value := false || 99
        short_circuit_and := false && missing_rhs
        short_circuit_or := true || missing_rhs_two

        contract_ok := falsey_hits == 0
            && truthy_hits == 7
            && and_value == true
            && or_value == true
            && short_circuit_and == false
            && short_circuit_or == true
    "#;

    assert_interpreter_and_vm_env_bool(script, "contract_ok");
}

#[test]
fn spec_indexing_and_invalid_operation_errors_are_deterministic() {
    let missing_map_key = r#"
        profile := {"name": "kujo"}
        return profile["missing"]
    "#;
    assert_interpreter_and_vm_error_contains(missing_map_key, "Missing map key");

    let invalid_key_type = r#"
        profile := {"name": "kujo"}
        return profile[true]
    "#;
    assert_interpreter_and_vm_error_contains(invalid_key_type, "Invalid index operation");

    let out_of_bounds = r#"
        values := [1]
        return values[5]
    "#;
    assert_interpreter_and_vm_error_contains(out_of_bounds, "Index out of bounds");
}
