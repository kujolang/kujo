use kujo::ast::{Expr, Stmt};
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::{ParseOutput, Parser};

fn parse_output(source: &str) -> ParseOutput {
    let tokens = tokenize(source).expect("test source should tokenize");
    let mut parser = Parser::new(tokens);
    parser.parse_with_diagnostics()
}

fn parse_single_statement(source: &str) -> Stmt {
    let output = parse_output(source);
    assert!(
        output.diagnostics.is_empty(),
        "expected no parse diagnostics, got {:?}",
        output.diagnostics
    );
    assert_eq!(output.stmts.len(), 1, "expected one statement");
    output.stmts.into_iter().next().expect("expected one parsed statement")
}

fn parse_single_expr_shape(source: &str) -> String {
    match parse_single_statement(source) {
        Stmt::ExprStmt(expr) => expr_shape(&expr),
        other => panic!("expected expression statement, got {other:?}"),
    }
}

fn expr_shape(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(name) => name.clone(),
        Expr::Int(value) => value.to_string(),
        Expr::Float(value) => value.to_string(),
        Expr::String(value) => format!("\"{value}\""),
        Expr::Bool(value) => value.to_string(),
        Expr::UnaryOp { op, operand } => format!("({} {})", op, expr_shape(operand)),
        Expr::BinaryOp { left, op, right } => {
            format!("({} {} {})", op, expr_shape(left), expr_shape(right))
        }
        Expr::IndexAccess { object, index } => {
            format!("(index {} {})", expr_shape(object), expr_shape(index))
        }
        Expr::FieldAccess { object, field } => {
            format!("(field {} .{})", expr_shape(object), field)
        }
        Expr::Call { function, args } => {
            let rendered_args = args.iter().map(expr_shape).collect::<Vec<_>>().join(" ");
            format!("(call {} {})", expr_shape(function), rendered_args)
        }
        _ => format!("{expr:?}"),
    }
}

fn run_script(source: &str) -> Interpreter {
    let output = parse_output(source);
    assert!(output.diagnostics.is_empty(), "expected parse success, got {:?}", output.diagnostics);
    let mut interpreter = Interpreter::new();
    interpreter.eval_stmts(&output.stmts);
    interpreter
}

#[test]
fn parser_precedence_arithmetic_before_multiplicative() {
    let shape = parse_single_expr_shape("1 + 2 * 3\n");
    assert_eq!(shape, "(+ 1 (* 2 3))");
}

#[test]
fn parser_precedence_comparison_after_arithmetic() {
    let shape = parse_single_expr_shape("1 + 2 < 3 * 4\n");
    assert_eq!(shape, "(< (+ 1 2) (* 3 4))");
}

#[test]
fn parser_precedence_equality_after_comparison() {
    let shape = parse_single_expr_shape("1 < 2 == 3 < 4\n");
    assert_eq!(shape, "(== (< 1 2) (< 3 4))");
}

#[test]
fn parser_precedence_boolean_and_before_or() {
    let shape = parse_single_expr_shape("a || b && c\n");
    assert_eq!(shape, "(|| a (&& b c))");
}

#[test]
fn parser_precedence_unary_before_multiplicative() {
    let shape = parse_single_expr_shape("-a * !b\n");
    assert_eq!(shape, "(* (- a) (! b))");
}

#[test]
fn parser_precedence_parentheses_override_default_order() {
    let shape = parse_single_expr_shape("(1 + 2) * 3\n");
    assert_eq!(shape, "(* (+ 1 2) 3)");
}

#[test]
fn parser_assignment_rhs_preserves_expression_precedence() {
    match parse_single_statement("total := 1 + 2 * 3\n") {
        Stmt::Assign { target, value } => {
            assert_eq!(expr_shape(&target), "total");
            assert_eq!(expr_shape(&value), "(+ 1 (* 2 3))");
        }
        other => panic!("expected assignment statement, got {other:?}"),
    }
}

#[test]
fn parser_compound_assignment_lowers_to_binary_update() {
    match parse_single_statement("total += 1 * 2\n") {
        Stmt::Assign { target, value } => {
            assert_eq!(expr_shape(&target), "total");
            assert_eq!(expr_shape(&value), "(+ total (* 1 2))");
        }
        other => panic!("expected assignment statement, got {other:?}"),
    }
}

#[test]
fn parser_compound_assignment_supports_index_targets() {
    match parse_single_statement("scores[0] += 3\n") {
        Stmt::Assign { target, value } => {
            assert_eq!(expr_shape(&target), "(index scores 0)");
            assert_eq!(expr_shape(&value), "(+ (index scores 0) 3)");
        }
        other => panic!("expected assignment statement, got {other:?}"),
    }
}

#[test]
fn parser_rejects_chained_assignment() {
    let output = parse_output("a := b := 1\n");
    assert!(output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Chained assignment is not supported")));
}

#[test]
fn runtime_precedence_mixed_comparison_equality_and_boolean() {
    let interpreter = run_script("result := 1 < 2 == 3 < 4 && false || true\n");
    assert!(matches!(interpreter.env.get("result"), Some(Value::Bool(true))));
}

#[test]
fn runtime_compound_assignment_updates_bound_value() {
    let interpreter = run_script(
        "total := 4\n\
         total += 3 * 2\n",
    );
    assert!(matches!(interpreter.env.get("total"), Some(Value::Int(10))));
}
