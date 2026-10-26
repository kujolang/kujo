//! Public captured Environment maps remain readable and writable by Rust hosts.
use kujo::{
    interpreter::{Interpreter, Value},
    lexer::tokenize,
    parser::Parser,
};

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
