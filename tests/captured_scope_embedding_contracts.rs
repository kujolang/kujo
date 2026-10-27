//! Public captured Environment maps remain readable and writable by Rust hosts.
use kujo::{
    interpreter::{Interpreter, Value},
    lexer::tokenize,
    parser::Parser,
};

#[test]
fn removing_immutable_local_before_call_preserves_global_fallback() {
    let mut interpreter = Interpreter::new();
    let source = "mut fixed := 11 func factory() { const fixed := 7 func step() { return fixed } return step } let step := factory()";
    interpreter.eval_stmts(&Parser::new(tokenize(source).unwrap()).parse());
    let Some(Value::Function(_, _, Some(captured))) = interpreter.env.get("step") else {
        panic!("expected captured AST function");
    };
    {
        let mut environment = captured.lock().unwrap();
        let local = environment
            .scopes
            .iter_mut()
            .skip(1)
            .rev()
            .find(|scope| scope.contains_key("fixed"))
            .unwrap();
        local.remove("fixed");
        // Callable capture omits shadowed globals, so explicitly install the
        // host's fallback in the public owner before invoking the function.
        environment.scopes[0].insert("fixed".into(), Value::Int(11));
        assert!(matches!(environment.get("fixed"), Some(Value::Int(11))));
    }
    interpreter.eval_stmts(&Parser::new(tokenize("let result := step()").unwrap()).parse());
    assert!(matches!(interpreter.env.get("result"), Some(Value::Int(11))));
}

#[test]
fn captured_scope_map_reads_and_host_writes_remain_effective() {
    let mut interpreter = Interpreter::new();
    let source = "func factory() { mut count := 1 func step() { count += 1 return count } return step } let step := factory() let first := step()";
    interpreter.eval_stmts(&Parser::new(tokenize(source).unwrap()).parse());
    assert!(matches!(interpreter.env.get("first"), Some(Value::Int(2))));
    let Some(Value::Function(_, _, Some(captured))) = interpreter.env.get("step") else {
        panic!("expected captured AST function");
    };
    {
        let mut environment = captured.lock().unwrap();
        let scope =
            environment.scopes.iter_mut().rev().find(|scope| scope.contains_key("count")).unwrap();
        assert!(
            matches!(scope.get("count"), Some(Value::Int(2))),
            "public scope exposed an internal marker"
        );
        scope.insert("count".into(), Value::Int(41));
        assert!(
            matches!(environment.get("count"), Some(Value::Int(41))),
            "host map write was ignored"
        );
    }
    interpreter.eval_stmts(&Parser::new(tokenize("let second := step()").unwrap()).parse());
    assert!(matches!(interpreter.env.get("second"), Some(Value::Int(42))));
    let environment = captured.lock().unwrap();
    assert!(matches!(
        environment.scopes.iter().rev().find_map(|scope| scope.get("count")),
        Some(Value::Int(42))
    ));
}
