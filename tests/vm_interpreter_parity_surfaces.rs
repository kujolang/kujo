use kujo::compiler::Compiler;
use kujo::interpreter::{Environment, Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::{VmExecutionResult, VM};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        "expected interpreter error containing {expected:?}, got {interp_error:?}"
    );

    let vm_env = vm_env_with_builtins();
    let vm_error = run_vm(script, vm_env).expect_err("expected VM execution to report an error");
    assert!(
        vm_error.contains(expected),
        "expected VM error containing {expected:?}, got {vm_error:?}"
    );
}

fn unique_spawn_key() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    format!("vm_interp_parity_spawn_{}_{}", std::process::id(), nanos)
}

fn unique_module_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    format!("vm_interp_parity_module_{}_{}", std::process::id(), nanos)
}

#[test]
fn vm_and_interpreter_match_struct_method_behavior_contract() {
    let script = r#"
        struct Vec2 {
            x: float,
            y: float,

            func doubled(self) {
                return Vec2 { x: self.x * 2.0, y: self.y * 2.0 }
            }
        }

        vector := Vec2 { x: 2.0, y: 3.0 }
        doubled := vector.doubled()
        struct_x := doubled.x
        struct_y := doubled.y
        struct_ok := struct_x == 4.0 && struct_y == 6.0
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "unexpected interpreter struct-method behavior: {:?}",
        interp.return_value
    );
    assert!(
        matches!(interp.env.get("struct_ok"), Some(Value::Bool(true))),
        "interpreter struct values: struct_x={:?}, struct_y={:?}, struct_ok={:?}",
        interp.env.get("struct_x"),
        interp.env.get("struct_y"),
        interp.env.get("struct_ok")
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "unexpected vm struct-method behavior: {vm_result:?}");
    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(matches!(vm_globals.get("struct_ok"), Some(Value::Bool(true))));
}

#[test]
fn vm_and_interpreter_match_legacy_method_without_self_field_lookup() {
    let script = r#"
        struct Test {
            x: float,

            func normal_method() {
                return x * 2.0
            }
        }

        t := Test { x: 5.0 }
        method_value := t.normal_method()
        method_ok := method_value == 10.0
    "#;

    assert_interpreter_and_vm_bool(script, "method_ok");
}

#[test]
fn vm_and_interpreter_match_struct_method_named_chain_collision_surface() {
    let script = r#"
        struct Calculator {
            base: float,

            func add(self, x) {
                return self.base + x
            }

            func chain(self, x) {
                temp := self.add(x)
                return temp * 2.0
            }
        }

        calc := Calculator { base: 10.0 }
        chain_ok := calc.chain(5.0) == 30.0
    "#;

    assert_interpreter_and_vm_bool(script, "chain_ok");
}

#[test]
fn vm_and_interpreter_match_struct_op_add_overload_surface() {
    let script = r#"
        struct Number {
            value: float,

            func op_add(other) {
                return value + other.value
            }
        }

        n1 := Number { value: 10.0 }
        n2 := Number { value: 20.0 }
        sum := n1 + n2
        op_add_ok := sum == 30.0
    "#;

    assert_interpreter_and_vm_bool(script, "op_add_ok");
}

#[test]
fn vm_and_interpreter_match_top_level_function_hoisting_surface() {
    let script = r#"
        result := call_before(3)

        func call_before(value) {
            return value + 1
        }

        hoist_ok := result == 4
    "#;

    assert_interpreter_and_vm_bool(script, "hoist_ok");
}

#[test]
fn vm_and_interpreter_match_spread_destructuring_surface() {
    let script = r#"
        values := [1, 2, 3, 4]
        let [first, second, third, fourth] := values
        profile := {"a": 1, "b": 2, "c": 3}
        let {a, b, c} := profile
        destructuring_ok := first == 1 && second == 2 && third == 3 && fourth == 4 && a == 1 && b == 2 && c == 3
        spread_array := [0, ...values, 9]
        spread_dict := {"x": 0, ...profile, "z": 9}
        spread_ok := spread_array[2] == 2 && spread_dict["b"] == 2
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter returned runtime error: {:?}",
        interp.return_value
    );
    assert!(matches!(interp.env.get("destructuring_ok"), Some(Value::Bool(true))));
    assert!(matches!(interp.env.get("spread_ok"), Some(Value::Bool(true))));

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "vm execution failed: {:?}", vm_result.err());

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(
        matches!(vm_globals.get("destructuring_ok"), Some(Value::Bool(true))),
        "vm destructuring values: first={:?}, second={:?}, third={:?}, fourth={:?}, a={:?}, b={:?}, c={:?}, destructuring_ok={:?}",
        vm_globals.get("first"),
        vm_globals.get("second"),
        vm_globals.get("third"),
        vm_globals.get("fourth"),
        vm_globals.get("a"),
        vm_globals.get("b"),
        vm_globals.get("c"),
        vm_globals.get("destructuring_ok")
    );
    assert!(matches!(vm_globals.get("spread_ok"), Some(Value::Bool(true))));
}

#[test]
fn vm_and_interpreter_match_dict_override_and_dict_method_surface() {
    let script = r#"
        base := {"x": 1, "y": 2}
        override := {"y": 99, "z": 100}
        merged_spread := {...base, ...override}
        spread_override_ok := merged_spread["x"] == 1 && merged_spread["y"] == 99 && merged_spread["z"] == 100

        defaults := {"timeout": 30, "retry": 3, "debug": false}
        config := {...defaults, "timeout": 60, "debug": true}
        literal_override_ok := config["timeout"] == 60 && config["debug"] == true && config["retry"] == 3

        dict1 := {"a": 1, "b": 2, "c": 3}
        dict2 := {"b": 7, "d": 4}
        merged_method := merge(dict1, dict2)
        merge_ok := merged_method["a"] == 1 && merged_method["b"] == 7 && merged_method["d"] == 4

        cleared := clear(dict1)
        clear_ok := len(cleared) == 0

        removed := remove(dict1, "b")
        remove_ok := removed[1] == 2 && len(removed[0]) == 2 && has_key(removed[0], "a") == 1 && has_key(removed[0], "b") == 0

        parity_ok := spread_override_ok && literal_override_ok && merge_ok && clear_ok && remove_ok
    "#;

    assert_interpreter_and_vm_bool(script, "parity_ok");
}

#[test]
fn vm_and_interpreter_match_null_optional_and_pipe_operator_surface() {
    let script = r#"
        func double(x) {
            return x * 2
        }

        func add_ten(x) {
            return x + 10
        }

        func square(x) {
            return x * x
        }

        value1 := null ?? "default"
        value2 := "actual" ?? "default"
        value3 := null ?? null ?? "fallback"
        value4 := null ?? "first" ?? "second"
        num1 := null ?? 42
        num2 := 10 ?? 42

        dict1 := {"name": "Bob", "age": 30}
        null_dict := null
        name1 := dict1?.name
        missing := dict1?.missing_field
        name2 := null_dict?.name
        default_name := null_dict?.name ?? "Anonymous"
        actual_name := dict1?.name ?? "Anonymous"

        result1 := 5 |> double
        result2 := 3 |> double |> add_ten |> square

        ops_ok :=
            value1 == "default" &&
            value2 == "actual" &&
            value3 == "fallback" &&
            value4 == "first" &&
            num1 == 42 &&
            num2 == 10 &&
            name1 == "Bob" &&
            missing == null &&
            name2 == null &&
            default_name == "Anonymous" &&
            actual_name == "Bob" &&
            result1 == 10 &&
            result2 == 256
    "#;

    assert_interpreter_and_vm_bool(script, "ops_ok");
}

#[test]
fn vm_and_interpreter_match_struct_unary_overload_surface() {
    let script = r#"
        struct Vector {
            x: float,
            y: float,

            func op_neg() {
                return Vector { x: -x, y: -y }
            }
        }

        struct Flag {
            value: bool,

            func op_not() {
                return Flag { value: !value }
            }
        }

        v := Vector { x: 3.0, y: 4.0 }
        neg_v := -v

        f := Flag { value: true }
        not_f := !f
        double_not := !!f

        unary_ok :=
            neg_v.x == -3.0 &&
            neg_v.y == -4.0 &&
            not_f.value == false &&
            double_not.value == true
    "#;

    assert_interpreter_and_vm_bool(script, "unary_ok");
}

#[test]
fn vm_and_interpreter_match_enum_match_binding_surface() {
    let script = r#"
        result_value := Result::Ok(42)
        label_value := Option::Some("ready")

        match result_value {
            case Result::Ok(v): { matched_ok := v }
            case Result::Err(e): { matched_ok := -1 }
            default: { matched_ok := -999 }
        }

        match label_value {
            case Option::Some(text): { matched_label := text }
            case Option::None: { matched_label := "none" }
            default: { matched_label := "none" }
        }

        match_ok := matched_ok == 42 && matched_label == "ready"
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter returned runtime error: {:?}",
        interp.return_value
    );
    assert!(
        matches!(interp.env.get("matched_ok"), Some(Value::Int(42)))
            && matches!(interp.env.get("matched_label"), Some(Value::Str(v)) if v.as_ref() == "ready")
            && matches!(interp.env.get("match_ok"), Some(Value::Bool(true))),
        "expected interpreter tag-style match binding support, got matched_ok={:?}, matched_label={:?}, match_ok={:?}",
        interp.env.get("matched_ok"),
        interp.env.get("matched_label"),
        interp.env.get("match_ok")
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "vm execution failed: {:?}", vm_result.err());

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(
        matches!(vm_globals.get("matched_ok"), Some(Value::Int(42)))
            && matches!(vm_globals.get("matched_label"), Some(Value::Str(v)) if v.as_ref() == "ready")
            && matches!(vm_globals.get("match_ok"), Some(Value::Bool(true))),
        "expected VM tag-style match binding support, got matched_ok={:?}, matched_label={:?}, match_ok={:?}",
        vm_globals.get("matched_ok"),
        vm_globals.get("matched_label"),
        vm_globals.get("match_ok")
    );
}

#[test]
fn vm_and_interpreter_match_custom_enum_constructor_calls_without_recursion() {
    let script = r#"
        enum Result {
            Ok,
            Err
        }

        func check(num) {
            if num > 0 {
                return Result::Ok("yes")
            }
            return Result::Err("no")
        }

        outcome := check(1)
        match outcome {
            case Result::Ok(msg): { constructor_ok := msg == "yes" }
            case Result::Err(err): { constructor_ok := false }
            default: { constructor_ok := false }
        }
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter should evaluate custom enum constructors without recursion errors: {:?}",
        interp.return_value
    );
    assert!(
        matches!(interp.env.get("constructor_ok"), Some(Value::Bool(true))),
        "interpreter custom enum constructor result mismatch: constructor_ok={:?}",
        interp.env.get("constructor_ok")
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "vm execution failed: {:?}", vm_result.err());

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(
        matches!(vm_globals.get("constructor_ok"), Some(Value::Bool(true))),
        "vm custom enum constructor result mismatch: constructor_ok={:?}",
        vm_globals.get("constructor_ok")
    );
}

#[test]
fn vm_and_interpreter_error_on_missing_string_map_key() {
    let script = r#"
        profile := {"name": "kujo"}
        return profile["missing"]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Missing map key");
}

#[test]
fn vm_and_interpreter_error_on_missing_integer_map_key() {
    let script = r#"
        scores := {}
        scores[0] := 10
        return scores[1]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Missing map key");
}

#[test]
fn vm_and_interpreter_error_on_nested_missing_map_key() {
    let script = r#"
        outer := {"inner": {"present": 7}}
        return outer["inner"]["missing"]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Missing map key");
}

#[test]
fn vm_and_interpreter_error_on_invalid_map_key_type() {
    let script = r#"
        profile := {"name": "kujo"}
        return profile[true]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid index operation");
}

#[test]
fn vm_and_interpreter_error_on_out_of_bounds_array_index() {
    let script = r#"
        values := [10, 20]
        return values[5]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Index out of bounds");
}

#[test]
fn vm_and_interpreter_error_on_out_of_bounds_string_index() {
    let script = r#"
        label := "kujo"
        return label[10]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Index out of bounds");
}

#[test]
fn vm_and_interpreter_error_on_indexing_non_indexable_value() {
    let script = r#"
        value := 42
        return value[0]
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid index operation");
}

#[test]
fn vm_and_interpreter_error_on_invalid_index_assignment_target() {
    let script = r#"
        value := 42
        value[0] := 7
        return value
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid index assignment");
}

#[test]
fn vm_and_interpreter_error_on_unsupported_binary_operation() {
    let script = r#"
        return "left" - "right"
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid binary operation");
}

#[test]
fn vm_and_interpreter_error_on_unsupported_unary_operation() {
    let script = r#"
        return -true
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid unary operation");
}

#[test]
fn vm_and_interpreter_error_on_break_outside_loop() {
    let script = "break\n";
    assert_interpreter_and_vm_error_contains(script, "break can only be used inside a loop");
}

#[test]
fn vm_and_interpreter_error_on_continue_outside_loop() {
    let script = "continue\n";
    assert_interpreter_and_vm_error_contains(script, "continue can only be used inside a loop");
}

#[test]
fn vm_and_interpreter_allow_break_and_continue_inside_loop() {
    let script = r#"
        counter := 0
        loop {
            counter := counter + 1
            if counter == 2 { continue }
            if counter == 4 { break }
        }
        loop_ok := counter == 4
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "unexpected interpreter control-flow error: {:?}",
        interp.return_value
    );
    assert!(matches!(interp.env.get("loop_ok"), Some(Value::Bool(true))));

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "unexpected vm control-flow error: {:?}", vm_result.err());
    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(matches!(vm_globals.get("loop_ok"), Some(Value::Bool(true))));
}

#[test]
fn vm_and_interpreter_error_on_break_outside_loop_inside_function() {
    let script = r#"
        func bad() {
            break
        }

        bad()
    "#;

    assert_interpreter_and_vm_error_contains(script, "break can only be used inside a loop");
}

#[test]
fn vm_and_interpreter_error_on_continue_outside_loop_inside_function() {
    let script = r#"
        func bad() {
            continue
        }

        bad()
    "#;

    assert_interpreter_and_vm_error_contains(script, "continue can only be used inside a loop");
}

#[test]
fn vm_and_interpreter_allow_top_level_return_for_script_exit() {
    let script = r#"
        value := 41
        return value + 1
    "#;

    let interp = run_interpreter(script);
    assert!(matches!(
        interp.return_value,
        Some(Value::Return(value)) if matches!(*value, Value::Int(42))
    ));

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env);
    assert!(
        vm_result.is_ok(),
        "expected VM to keep top-level return script behavior, got {vm_result:?}"
    );
}

#[test]
fn vm_and_interpreter_match_valid_index_assignment_success_path() {
    let script = r#"
        values := [2, 4, 6]
        values[1] := values[1] + 3
        index_assignment_ok := values[1] == 7
    "#;

    assert_interpreter_and_vm_bool(script, "index_assignment_ok");
}

#[test]
fn vm_and_interpreter_error_on_undefined_top_level_identifier() {
    let script = r#"
        return missing_top_level_name
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_top_level_name");
}

#[test]
fn vm_and_interpreter_error_on_undefined_identifier_in_binary_expression() {
    let script = r#"
        return missing_operand + 1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_operand");
}

#[test]
fn vm_and_interpreter_error_on_undefined_identifier_in_condition() {
    let script = r#"
        if missing_condition {
            condition_reached := true
        }
        return false
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_condition");
}

#[test]
fn vm_and_interpreter_error_on_undefined_identifier_inside_function() {
    let script = r#"
        func read_missing() {
            return missing_inside_function
        }

        return read_missing()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_inside_function");
}

#[test]
fn vm_and_interpreter_error_on_undefined_identifier_inside_closure() {
    let script = r#"
        func make_reader() {
            captured := 1
            reader := func() {
                return missing_inside_closure
            }
            return reader
        }

        read := make_reader()
        return read()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_inside_closure");
}

#[test]
fn vm_and_interpreter_error_on_undefined_method_receiver() {
    let script = r#"
        return missing_receiver.collect()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: missing_receiver");
}

#[test]
fn vm_and_interpreter_error_on_unknown_method_member() {
    let script = r#"
        values := [1, 2, 3]
        return values.definitely_missing_method()
    "#;

    assert_interpreter_and_vm_error_contains(script, "method");
}

#[test]
fn vm_and_interpreter_error_on_unsupported_struct_generator_method() {
    let script = r#"
        struct Counter {
            value: int,

            func* emit(self) {
                yield self.value
            }
        }

        counter := Counter { value: 7 }
        return counter.emit()
    "#;

    assert_interpreter_and_vm_error_contains(
        script,
        "Generator methods are not supported for structs: Counter.emit",
    );
}

#[test]
fn vm_and_interpreter_error_on_function_arity_too_few() {
    let script = r#"
        func add(a, b) {
            return a + b
        }

        return add(1)
    "#;

    assert_interpreter_and_vm_error_contains(script, "add expects 2 arguments, got 1");
}

#[test]
fn vm_and_interpreter_error_on_function_arity_too_many() {
    let script = r#"
        func add(a, b) {
            return a + b
        }

        return add(1, 2, 3)
    "#;

    assert_interpreter_and_vm_error_contains(script, "add expects 2 arguments, got 3");
}

#[test]
fn vm_and_interpreter_error_on_closure_arity_mismatch() {
    let script = r#"
        multiplier := func(value) {
            return value * 2
        }

        return multiplier()
    "#;

    assert_interpreter_and_vm_error_contains(script, "expects 1 arguments, got 0");
}

#[test]
fn vm_and_interpreter_error_on_closure_arity_too_many() {
    let script = r#"
        multiplier := func(value) {
            return value * 2
        }

        return multiplier(1, 2)
    "#;

    assert_interpreter_and_vm_error_contains(script, "expects 1 arguments, got 2");
}

#[test]
fn vm_and_interpreter_error_on_method_arity_mismatch() {
    let script = r#"
        struct Counter {
            count: int,

            func bump(self, delta) {
                return self.count + delta
            }
        }

        counter := Counter { count: 4 }
        return counter.bump()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Counter.bump expects 1 arguments, got 0");
}

#[test]
fn vm_and_interpreter_error_on_method_arity_too_many() {
    let script = r#"
        struct Counter {
            count: int,

            func bump(self, delta) {
                return self.count + delta
            }
        }

        counter := Counter { count: 4 }
        return counter.bump(1, 2)
    "#;

    assert_interpreter_and_vm_error_contains(script, "Counter.bump expects 1 arguments, got 2");
}

#[test]
fn vm_and_interpreter_error_on_async_function_arity_mismatch() {
    let script = r#"
        async func combine(a, b) {
            return a + b
        }

        promise := combine(1)
        return await promise
    "#;

    assert_interpreter_and_vm_error_contains(script, "combine expects 2 arguments, got 1");
}

#[test]
fn vm_and_interpreter_error_on_async_function_arity_too_many() {
    let script = r#"
        async func combine(a, b) {
            return a + b
        }

        promise := combine(1, 2, 3)
        return await promise
    "#;

    assert_interpreter_and_vm_error_contains(script, "combine expects 2 arguments, got 3");
}

#[test]
fn vm_and_interpreter_error_on_generator_arity_mismatch() {
    let script = r#"
        func* emit_twice(value) {
            yield value
            yield value
        }

        total := 0
        for item in emit_twice() {
            total := total + item
        }

        return total
    "#;

    assert_interpreter_and_vm_error_contains(script, "emit_twice expects 1 arguments, got 0");
}

#[test]
fn vm_and_interpreter_error_on_generator_arity_too_many() {
    let script = r#"
        func* emit_twice(value) {
            yield value
            yield value
        }

        total := 0
        for item in emit_twice(1, 2) {
            total := total + item
        }

        return total
    "#;

    assert_interpreter_and_vm_error_contains(script, "emit_twice expects 1 arguments, got 2");
}

#[test]
fn vm_and_interpreter_match_generator_iteration_surface() {
    let script = r#"
        func* emit_numbers(start) {
            yield start
            yield start + 1
            yield start + 2
        }

        sum := 0
        count := 0
        for item in emit_numbers(5) {
            sum := sum + item
            count := count + 1
        }

        generator_surface_ok := count == 3 && sum == 18
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter should support top-level generator iteration: {:?}",
        interp.return_value
    );
    assert!(
        matches!(interp.env.get("generator_surface_ok"), Some(Value::Bool(true))),
        "interpreter generator result should be true, got {:?}",
        interp.env.get("generator_surface_ok")
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env);
    assert!(
        vm_result.is_ok(),
        "vm should support top-level generator iteration surface, got {vm_result:?}"
    );

    let vm_env = vm_env_with_builtins();
    run_vm(script, vm_env.clone()).expect("vm script should complete");
    let vm_env_after = vm_env.lock().expect("failed to lock vm globals");
    assert!(
        matches!(vm_env_after.get("generator_surface_ok"), Some(Value::Bool(true))),
        "vm generator result should be true, got {:?}",
        vm_env_after.get("generator_surface_ok")
    );
}

#[test]
fn vm_and_interpreter_error_on_native_function_arity_mismatch() {
    let script = r#"
        return len("kujo", "extra")
    "#;

    assert_interpreter_and_vm_error_contains(script, "len expects 1 arguments, got 2");
}

#[test]
fn vm_and_interpreter_error_on_native_function_arity_too_few() {
    let script = r#"
        return len()
    "#;

    assert_interpreter_and_vm_error_contains(script, "len expects 1 arguments, got 0");
}

#[test]
fn vm_and_interpreter_error_on_range_native_arity() {
    let script = r#"
        return input("prompt", "extra")
    "#;

    assert_interpreter_and_vm_error_contains(script, "input expects 0 to 1 arguments, got 2");
}

#[test]
fn vm_and_interpreter_match_callable_arity_success_paths() {
    let script = r#"
        func add(a, b) {
            return a + b
        }

        closure := func(value) {
            return value * 2
        }

        struct Counter {
            count: int,

            func bump(self, delta) {
                return self.count + delta
            }
        }

        async func async_add(a, b) {
            return a + b
        }

        counter := Counter { count: 10 }
        async_result := await async_add(4, 5)
        arity_success_ok := add(1, 2) == 3
            && closure(5) == 10
            && counter.bump(7) == 17
            && async_result == 9
    "#;

    assert_interpreter_and_vm_bool(script, "arity_success_ok");
}

#[test]
fn vm_and_interpreter_preserve_variadic_native_contracts() {
    let script = r#"
        debug("single")
        debug("a", "b", "c")
        print("variadic-path-ok")
    "#;

    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter returned runtime error for variadic native call: {:?}",
        interp.return_value
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env);
    assert!(
        vm_result.is_ok(),
        "vm returned runtime error for variadic native call: {:?}",
        vm_result.err()
    );
}

fn assert_interpreter_and_vm_bool(script: &str, flag_name: &str) {
    let interp = run_interpreter(script);
    assert!(
        interp.return_value.is_none(),
        "interpreter returned runtime error: {:?}",
        interp.return_value
    );
    assert!(matches!(interp.env.get(flag_name), Some(Value::Bool(true))));

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(script, vm_env.clone());
    assert!(vm_result.is_ok(), "vm execution failed: {:?}", vm_result.err());

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(matches!(vm_globals.get(flag_name), Some(Value::Bool(true))));
}

#[test]
fn vm_and_interpreter_match_bounded_namespace_aware_xml_parsing() {
    let script = r#"
        parsed := parse_xml_bounded(
            "<feedback xmlns=\"urn:ietf:params:xml:ns:dmarc-2.0\"><record id=\"1\"><domain>example.com</domain></record></feedback>",
            {"max_input_bytes": 4096, "max_depth": 8, "max_nodes": 8, "max_attributes": 8, "max_text_bytes": 1024, "max_tree_bytes": 4096}
        )
        xml_parity_ok := parsed["ok"] &&
            parsed["code"] == "XML_DOCUMENT_PARSED" &&
            parsed["node_count"] == 3 &&
            parsed["attribute_count"] == 2 &&
            parsed["tree_bytes"] > 0 &&
            parsed["root"]["name"] == "feedback" &&
            parsed["root"]["namespace"] == "urn:ietf:params:xml:ns:dmarc-2.0"
    "#;

    assert_interpreter_and_vm_bool(script, "xml_parity_ok");
}

#[test]
fn vm_and_interpreter_keep_string_literals_explicit() {
    let script = r#"
        literal := "missing_top_level_name"
        literal_ok := literal == "missing_top_level_name"
    "#;

    assert_interpreter_and_vm_bool(script, "literal_ok");
}

#[test]
fn vm_and_interpreter_resolve_defined_identifiers() {
    let script = r#"
        top := 10

        func add_one(value) {
            local := value + 1
            return local
        }

        func make_adder(seed) {
            return func(delta) {
                return seed + delta
            }
        }

        add_seed := make_adder(top)
        identifiers_ok := add_one(top) == 11 && add_seed(5) == 15
    "#;

    assert_interpreter_and_vm_bool(script, "identifiers_ok");
}

#[test]
fn vm_and_interpreter_match_successful_local_map_update() {
    let script = r#"
        counts := {"hits": 1}
        counts["hits"] := counts["hits"] + 1

        numeric := {}
        numeric[0] := 10
        numeric[0] := numeric[0] + 5

        map_update_ok := counts["hits"] == 2 && numeric[0] == 15
    "#;

    assert_interpreter_and_vm_bool(script, "map_update_ok");
}

#[test]
fn vm_and_interpreter_match_successful_nested_map_update() {
    let script = r#"
        nested := {"inner": {"value": 2}}
        inner := nested["inner"]
        inner["value"] := inner["value"] + 3
        nested["inner"] := inner

        map_update_ok := nested["inner"]["value"] == 5
    "#;

    assert_interpreter_and_vm_bool(script, "map_update_ok");
}

#[test]
fn vm_and_interpreter_match_successful_captured_map_update() {
    let script = r#"
        func make_bump() {
            captured := {"count": 4}
            bump := func() {
                captured["count"] := captured["count"] + 6
                return captured["count"]
            }
            return bump
        }

        bump := make_bump()
        bumped := bump()

        map_update_ok := bumped == 10
    "#;

    assert_interpreter_and_vm_bool(script, "map_update_ok");
}

#[test]
fn vm_and_interpreter_match_named_nested_capture_mutation() {
    let script = r#"
        func make_counter() {
            mut count := 0

            func bump() {
                count := count + 1
                return count
            }

            return bump
        }

        bump := make_counter()
        first := bump()
        second := bump()

        capture_mutation_ok := first == 1 && second == 2
    "#;

    assert_interpreter_and_vm_bool(script, "capture_mutation_ok");
}

#[test]
fn vm_and_interpreter_match_async_named_nested_capture_mutation() {
    let script = r#"
        func make_counter() {
            mut count := 0

            async func bump() {
                count := count + 1
                return count
            }

            return bump
        }

        bump := make_counter()
        first := await bump()
        second := await bump()

        async_capture_mutation_ok := first == 1 && second == 2
    "#;

    assert_interpreter_and_vm_bool(script, "async_capture_mutation_ok");
}

#[test]
fn vm_and_interpreter_match_async_named_nested_capture_isolation() {
    let script = r#"
        func make_counter(start) {
            mut count := start

            async func bump(delta) {
                count := count + delta
                return count
            }

            return bump
        }

        left := make_counter(0)
        right := make_counter(10)
        left_one := await left(1)
        right_one := await right(5)
        left_two := await left(2)
        right_two := await right(1)

        async_capture_isolation_ok := left_one == 1
            && left_two == 3
            && right_one == 15
            && right_two == 16
    "#;

    assert_interpreter_and_vm_bool(script, "async_capture_isolation_ok");
}

#[test]
fn vm_and_interpreter_match_import_export_surface() {
    let module_name = unique_module_name();
    let module_filename = format!("{module_name}.kujo");
    let module_source = "export answer := 42\n";
    fs::write(&module_filename, module_source).expect("failed to write parity module");

    let script = format!(
        r#"
        from {module_name} import answer
        import_ok := answer == 42
    "#
    );

    assert_interpreter_and_vm_bool(&script, "import_ok");
    let _ = fs::remove_file(module_filename);
}

#[test]
fn vm_and_interpreter_match_imported_function_invocation_surface() {
    let module_name = unique_module_name();
    let module_filename = format!("{module_name}.kujo");
    let module_source = r#"
export func add_one(x) {
    return x + 1
}
"#;
    fs::write(&module_filename, module_source).expect("failed to write parity module");

    let script = format!(
        r#"
        from {module_name} import add_one
        import_function_ok := add_one(41) == 42
    "#
    );

    assert_interpreter_and_vm_bool(&script, "import_function_ok");
    let _ = fs::remove_file(module_filename);
}

#[test]
fn vm_and_interpreter_match_import_all_function_invocation_surface() {
    let module_name = unique_module_name();
    let module_filename = format!("{module_name}.kujo");
    let module_source = r#"
export func add_one(x) {
    return x + 1
}
"#;
    fs::write(&module_filename, module_source).expect("failed to write parity module");

    let script = format!(
        r#"
        import {module_name}
        import_all_function_ok := add_one(41) == 42
    "#
    );

    assert_interpreter_and_vm_bool(&script, "import_all_function_ok");
    let _ = fs::remove_file(module_filename);
}

#[test]
fn vm_and_interpreter_match_imported_function_dict_index_surface() {
    let module_name = unique_module_name();
    let module_filename = format!("{module_name}.kujo");
    let module_source = r#"
export func read_opts(opts) {
    return opts["quiet"] == true && opts["parallel_workers"] == 2
}
"#;
    fs::write(&module_filename, module_source).expect("failed to write parity module");

    let script = format!(
        r#"
        from {module_name} import read_opts
        options := {{"quiet": true, "parallel_workers": 2}}
        import_function_dict_ok := read_opts(options)
    "#
    );

    assert_interpreter_and_vm_bool(&script, "import_function_dict_ok");
    let _ = fs::remove_file(module_filename);
}

#[test]
fn vm_and_interpreter_match_dotted_from_import_surface() {
    let root_module = unique_module_name();
    let nested_dir = format!("modules/{root_module}/core");
    fs::create_dir_all(&nested_dir).expect("failed to create nested parity module dir");
    let module_filename = format!("{nested_dir}/math.kujo");
    let module_source = "export answer := 42\n";
    fs::write(&module_filename, module_source).expect("failed to write nested parity module");

    let script = format!(
        r#"
        from {root_module}.core.math import answer
        dotted_import_ok := answer == 42
    "#
    );

    assert_interpreter_and_vm_bool(&script, "dotted_import_ok");
    let _ = fs::remove_dir_all(format!("modules/{root_module}"));
}

#[test]
fn vm_and_interpreter_match_dotted_imported_function_invocation_surface() {
    let root_module = unique_module_name();
    let nested_dir = format!("modules/{root_module}/core");
    fs::create_dir_all(&nested_dir).expect("failed to create nested parity module dir");
    let module_filename = format!("{nested_dir}/math.kujo");
    let module_source = r#"
export func add_one(x) {
    return x + 1
}
"#;
    fs::write(&module_filename, module_source).expect("failed to write nested parity module");

    let script = format!(
        r#"
        from {root_module}.core.math import add_one
        dotted_import_function_ok := add_one(41) == 42
    "#
    );

    assert_interpreter_and_vm_bool(&script, "dotted_import_function_ok");
    let _ = fs::remove_dir_all(format!("modules/{root_module}"));
}

#[test]
fn vm_and_interpreter_dotted_import_resolution_prefers_flat_module_before_nested_path() {
    let root_module = unique_module_name();
    let nested_dir = format!("modules/{root_module}/core");
    fs::create_dir_all(&nested_dir).expect("failed to create nested parity module dir");

    let dotted_name = format!("{root_module}.core.math");
    let flat_module_path = format!("modules/{dotted_name}.kujo");
    fs::write(&flat_module_path, "export source := \"flat\"\n")
        .expect("failed to write flat dotted module file");
    fs::write(format!("{nested_dir}/math.kujo"), "export source := \"nested\"\n")
        .expect("failed to write nested dotted module file");

    let script = format!(
        r#"
        from {dotted_name} import source
        dotted_precedence_ok := source == "flat"
    "#
    );

    assert_interpreter_and_vm_bool(&script, "dotted_precedence_ok");
    let _ = fs::remove_file(flat_module_path);
    let _ = fs::remove_dir_all(format!("modules/{root_module}"));
}

#[test]
fn vm_and_interpreter_reject_integer_add_overflow() {
    let script = r#"
        return 9223372036854775807 + 1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Integer overflow");
}

#[test]
fn vm_and_interpreter_reject_integer_subtract_overflow() {
    let script = r#"
        minimum := parse_int("-9223372036854775808")
        return minimum - 1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Integer overflow");
}

#[test]
fn vm_and_interpreter_reject_integer_multiply_overflow() {
    let script = r#"
        return 3037000500 * 3037000500
    "#;

    assert_interpreter_and_vm_error_contains(script, "Integer overflow");
}

#[test]
fn vm_and_interpreter_reject_integer_division_overflow() {
    let script = r#"
        minimum := parse_int("-9223372036854775808")
        return minimum / -1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Integer overflow");
}

#[test]
fn vm_and_interpreter_reject_float_division_by_zero() {
    let script = r#"
        return 1.0 / 0.0
    "#;

    assert_interpreter_and_vm_error_contains(script, "Division by zero");
}

#[test]
fn vm_and_interpreter_reject_float_modulo_by_zero() {
    let script = r#"
        return 1.0 % 0.0
    "#;

    assert_interpreter_and_vm_error_contains(script, "Modulo by zero");
}

#[test]
fn vm_and_interpreter_nan_and_infinity_comparisons_match_policy() {
    let script = r#"
        nan_value := parse_float("NaN")
        pos_inf := parse_float("inf")
        neg_inf := parse_float("-inf")

        nan_ok := (nan_value == nan_value) == false && (nan_value != nan_value) == true
        inf_ok := pos_inf == parse_float("inf") && pos_inf > 1.0 && neg_inf < -1.0 && pos_inf != neg_inf

        numeric_policy_ok := nan_ok && inf_ok
    "#;

    assert_interpreter_and_vm_bool(script, "numeric_policy_ok");
}

#[test]
fn vm_and_interpreter_define_cross_type_numeric_and_string_ordering_contract() {
    let script = r#"
        int_float_eq := 1 == 1.0
        int_float_ne := 1 != 1.0
        int_float_lt := 1 < 1.5
        float_int_ge := 2.0 >= 2
        string_lt := "ant" < "bee"

        comparison_contract_ok := int_float_eq
            && (int_float_ne == false)
            && int_float_lt
            && float_int_ge
            && string_lt
    "#;

    assert_interpreter_and_vm_bool(script, "comparison_contract_ok");
}

#[test]
fn vm_and_interpreter_define_collection_and_callable_equality_contract() {
    let script = r#"
        func make_adder(seed) {
            return func(delta) {
                return seed + delta
            }
        }

        adder := make_adder(1)
        same_func := adder == adder
        other_func := adder == make_adder(1)
        native_eq := print == print

        array_eq := [1, 2, [3]] == [1, 2, [3]]
        array_ne := [1, 2] != [1, 2, 3]

        left := {"a": 1, "nested": {"x": 2}}
        right := {"nested": {"x": 2}, "a": 1}
        dict_eq := left == right
        dict_ne := left != {"a": 1, "nested": {"x": 3}}

        equality_contract_ok := same_func
            && (other_func == false)
            && native_eq
            && array_eq
            && array_ne
            && dict_eq
            && dict_ne
    "#;

    assert_interpreter_and_vm_bool(script, "equality_contract_ok");
}

#[test]
fn vm_and_interpreter_reject_boolean_ordering_comparisons() {
    let script = r#"
        func compare(left, right) {
            return left < right
        }

        return compare(true, false)
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid binary operation");
}

#[test]
fn vm_and_interpreter_reject_cross_type_ordering_comparisons() {
    let script = r#"
        return 1 < "1"
    "#;

    assert_interpreter_and_vm_error_contains(script, "Invalid binary operation");
}

#[test]
fn vm_and_interpreter_reject_overflow_in_local_in_place_addition() {
    let script = r#"
        mut total := 9223372036854775807
        total := total + 1
        return total
    "#;

    assert_interpreter_and_vm_error_contains(script, "Integer overflow");
}

#[test]
fn vm_and_interpreter_reject_reassignment_of_immutable_let_binding() {
    let script = r#"
        let value := 1
        value := 2
    "#;

    assert_interpreter_and_vm_error_contains(
        script,
        "Cannot reassign immutable let binding: value",
    );
}

#[test]
fn vm_and_interpreter_reject_reassignment_of_const_binding() {
    let script = r#"
        const answer := 41
        answer := 42
    "#;

    assert_interpreter_and_vm_error_contains(script, "Cannot reassign const binding: answer");
}

#[test]
fn vm_and_interpreter_reject_in_place_mutation_of_immutable_binding() {
    let script = r#"
        let counts := {"hits": 1}
        counts["hits"] := counts["hits"] + 1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Cannot mutate immutable let binding: counts");
}

#[test]
fn vm_and_interpreter_reject_in_place_mutation_of_const_binding() {
    let script = r#"
        const counts := {"hits": 1}
        counts["hits"] := counts["hits"] + 1
    "#;

    assert_interpreter_and_vm_error_contains(script, "Cannot mutate const binding: counts");
}

#[test]
fn vm_and_interpreter_allow_mutable_bindings_to_reassign_and_mutate() {
    let script = r#"
        mut total := 1
        total := total + 1

        mut counts := {"hits": 1}
        counts["hits"] := counts["hits"] + 2

        func mutate_local() {
            mut local_total := 3
            local_total := local_total + 4

            mut local_counts := {"hits": 10}
            local_counts["hits"] := local_counts["hits"] + 5

            return local_total == 7 && local_counts["hits"] == 15
        }

        mutable_ok := total == 2 && counts["hits"] == 3 && mutate_local()
    "#;

    assert_interpreter_and_vm_bool(script, "mutable_ok");
}

#[test]
fn vm_and_interpreter_reject_local_reassignment_of_immutable_let_binding() {
    let script = r#"
        func mutate_local() {
            let value := 10
            value := 11
            return value
        }

        mutate_local()
    "#;

    assert_interpreter_and_vm_error_contains(
        script,
        "Cannot reassign immutable let binding: value",
    );
}

#[test]
fn vm_and_interpreter_reject_reassignment_of_captured_immutable_let_binding() {
    let script = r#"
        func make_counter() {
            let count := 0
            return func() {
                count := count + 1
                return count
            }
        }

        counter := make_counter()
        counter()
    "#;

    assert_interpreter_and_vm_error_contains(
        script,
        "Cannot reassign immutable let binding: count",
    );
}

#[test]
fn vm_and_interpreter_allow_inner_scope_shadowing_without_leaking() {
    let script = r#"
        func check_shadowing() {
            let value := 10
            inner_seen := 0

            if true {
                let value := 25
                inner_seen := value
            }

            return value == 10 && inner_seen == 25
        }

        shadowing_ok := check_shadowing()
    "#;

    assert_interpreter_and_vm_bool(script, "shadowing_ok");
}

#[test]
fn vm_and_interpreter_assignment_updates_outer_binding_when_present() {
    let script = r#"
        total := 0

        func add_to_total(value) {
            total := total + value

            if value > 1 {
                total := total + 1
            }

            return total
        }

        first := add_to_total(2)
        second := add_to_total(3)
        outer_assign_ok := first == 3 && second == 7 && total == 7
    "#;

    assert_interpreter_and_vm_bool(script, "outer_assign_ok");
}

#[test]
fn vm_and_interpreter_reject_duplicate_let_declaration_in_same_scope() {
    let script = r#"
        let score := 1
        let score := 2
    "#;

    assert_interpreter_and_vm_error_contains(script, "Duplicate declaration in the same scope");
}

#[test]
fn vm_and_interpreter_reject_duplicate_const_declaration_in_same_scope() {
    let script = r#"
        const answer := 41
        const answer := 42
    "#;

    assert_interpreter_and_vm_error_contains(script, "Duplicate declaration in the same scope");
}

#[test]
fn vm_and_interpreter_allow_reusing_loop_local_names_in_sibling_while_scopes() {
    let script = r#"
        func check_loop_scope_reuse() {
            while true {
                mut idx := 0
                idx = idx + 1
                break
            }

            while true {
                mut idx := 0
                idx = idx + 2
                break
            }

            return true
        }

        loop_scope_reuse_ok := check_loop_scope_reuse()
    "#;

    assert_interpreter_and_vm_bool(script, "loop_scope_reuse_ok");
}

#[test]
fn vm_and_interpreter_reject_block_variable_leakage() {
    let script = r#"
        func leak_test() {
            if true {
                let inside := 1
            }

            return inside
        }

        leak_test()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: inside");
}

#[test]
fn vm_and_interpreter_reject_loop_variable_leakage() {
    let script = r#"
        func leak_test() {
            total := 0
            for item in [1, 2, 3] {
                total := total + item
            }

            return item
        }

        leak_test()
    "#;

    assert_interpreter_and_vm_error_contains(script, "Undefined variable: item");
}

#[test]
fn vm_and_interpreter_closure_captures_nearest_lexical_binding() {
    let script = r#"
        value := 5

        func make_reader() {
            let value := 9
            return func() {
                return value
            }
        }

        reader := make_reader()
        captured_ok := reader() == 9 && value == 5
    "#;

    assert_interpreter_and_vm_bool(script, "captured_ok");
}

#[test]
fn vm_and_interpreter_match_truthiness_semantics_across_conditionals() {
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

        func always_ready() {
            return true
        }
        if always_ready { truthy_hits += 1 }

        countdown := 3
        while_count := 0
        while countdown {
            while_count += 1
            countdown -= 1
        }

        float_gate := 0.5
        float_count := 0
        while float_gate {
            float_count += 1
            float_gate -= 0.5
        }

        string_false_branch := "unset"
        if "false" {
            string_false_branch := "truthy"
        } else {
            string_false_branch := "falsey"
        }

        logical_ok := (0 && 1) == false
            && (1 && 2) == true
            && (0 || 5) == true
            && ("" || "") == false
            && (null || "x") == true

        truthiness_ok := falsey_hits == 0
            && truthy_hits == 8
            && while_count == 3
            && float_count == 1
            && string_false_branch == "truthy"
            && logical_ok
    "#;

    assert_interpreter_and_vm_bool(script, "truthiness_ok");
}

#[test]
fn vm_and_interpreter_short_circuit_logical_operators_skip_rhs_when_possible() {
    let script = r#"
        and_short := false && missing_and_rhs
        or_short := true || missing_or_rhs
        short_circuit_ok := and_short == false && or_short == true
    "#;

    assert_interpreter_and_vm_bool(script, "short_circuit_ok");
}

#[test]
fn vm_and_interpreter_short_circuit_logical_operators_evaluate_rhs_when_required() {
    let and_rhs_required = r#"
        return true && missing_rhs_value
    "#;
    assert_interpreter_and_vm_error_contains(and_rhs_required, "Undefined variable");

    let or_rhs_required = r#"
        return 0 || missing_rhs_value
    "#;
    assert_interpreter_and_vm_error_contains(or_rhs_required, "Undefined variable");
}

#[test]
fn vm_and_interpreter_function_fallthrough_and_bare_return_yield_null() {
    let script = r#"
        func implicit_fallthrough() {
            marker := 1
        }

        func bare_return() {
            return
        }

        func explicit_zero() {
            return 0
        }

        implicit_value := implicit_fallthrough()
        bare_value := bare_return()
        zero_value := explicit_zero()

        falsey_hits := 0
        if implicit_value { falsey_hits += 1 }
        if bare_value { falsey_hits += 1 }

        null_return_ok := type(implicit_value) == "null"
            && type(bare_value) == "null"
            && implicit_value == null
            && bare_value == null
            && zero_value == 0
            && falsey_hits == 0
    "#;

    assert_interpreter_and_vm_bool(script, "null_return_ok");
}

#[test]
fn vm_and_interpreter_match_null_equality_contract() {
    let script = r#"
        eq_null := null == null
        ne_null := null != null
        eq_int_zero := null == 0
        ne_int_zero := null != 0
        eq_empty_string := null == ""
        ne_empty_string := null != ""

        null_eq_ok := eq_null == true
            && ne_null == false
            && eq_int_zero == false
            && ne_int_zero == true
            && eq_empty_string == false
            && ne_empty_string == true
    "#;

    assert_interpreter_and_vm_bool(script, "null_eq_ok");
}

#[test]
fn vm_and_interpreter_match_spawn_surface() {
    let spawn_key = unique_spawn_key();
    let script = format!(
        r#"
        shared_set("{spawn_key}", 0)
        spawn {{
            shared_add_int("{spawn_key}", 1)
        }}
        spawn_final := shared_get("{spawn_key}")
        spawn_ok := spawn_final >= 0
        shared_delete("{spawn_key}")
    "#
    );

    let interp = run_interpreter(&script);
    assert!(
        interp.return_value.is_none(),
        "interpreter returned runtime error: {:?}",
        interp.return_value
    );
    assert!(matches!(interp.env.get("spawn_ok"), Some(Value::Bool(true))));

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(&script, vm_env.clone());
    assert!(vm_result.is_ok(), "vm execution failed: {:?}", vm_result.err());

    let vm_globals = vm_env.lock().expect("failed to lock vm globals");
    assert!(matches!(vm_globals.get("spawn_ok"), Some(Value::Bool(true))));
}

#[test]
fn vm_and_interpreter_parallel_map_preserve_global_dependencies_and_order() {
    let script = r#"
        func dependency(value) {
            return {"ok": true, "value": value * 2}
        }

        async func worker(job) {
            await async_sleep(job["delay"])
            return dependency(job["value"])
        }

        mapped := await parallel_map([
            {"value": 1, "delay": 40},
            {"value": 2, "delay": 20},
            {"value": 3, "delay": 1}
        ], worker, 2)

        async func rejecting_worker(value) {
            if value == 2 { return error("provider-denied") }
            return value
        }
        caught_rejection := false
        try {
            await parallel_map([1, 2, 3], rejecting_worker, 2)
        } except err {
            caught_rejection = contains(err.message, "provider-denied") == 1
        }

        parallel_ok := len(mapped) == 3 && mapped[0]["ok"] == true && mapped[0]["value"] == 2 && mapped[1]["value"] == 4 && mapped[2]["value"] == 6 && caught_rejection
    "#;

    assert_interpreter_and_vm_bool(script, "parallel_ok");
}

#[test]
fn vm_and_interpreter_match_throw_call_stack_surface() {
    let script = r#"
        func safe_divide(a, b) {
            if b == 0 {
                throw("division by zero")
            }
            return a / b
        }

        throw_stack_ok := false
        throw_message_ok := false
        try {
            safe_divide(10, 0)
        } except err {
            throw_message_ok = err.message == "division by zero"
            throw_stack_ok = len(err.stack) > 0 && err.stack[0] == "safe_divide"
        }

        parity_ok := throw_message_ok && throw_stack_ok
    "#;

    assert_interpreter_and_vm_bool(script, "parity_ok");
}

#[test]
fn vm_and_interpreter_execute_exception_fixture_without_runtime_arity_drift() {
    let script = fs::read_to_string("tests/test_exceptions_comprehensive.kujo")
        .expect("expected exception fixture to be readable");

    let interp = run_interpreter(&script);
    assert!(
        interp.return_value.is_none(),
        "interpreter should complete exception fixture without runtime error: {:?}",
        interp.return_value
    );

    let vm_env = vm_env_with_builtins();
    let vm_result = run_vm(&script, vm_env);
    assert!(
        vm_result.is_ok(),
        "vm should complete exception fixture without arity drift/runtime failure: {vm_result:?}"
    );
}
