use kujo::compiler::Compiler;
use kujo::interpreter::Interpreter;
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};

fn run(source: &str, vm: bool) {
    let mut parser = Parser::new(tokenize(source).expect("tokenize"));
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut interpreter = Interpreter::new();
    if vm {
        let mut runtime = VM::new();
        runtime.set_globals(Arc::new(Mutex::new(interpreter.env)));
        runtime.execute(Compiler::new().compile(&parsed.stmts).expect("compile")).expect("VM");
    } else {
        interpreter.eval_stmts(&parsed.stmts);
        assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    }
}

const FIELDS: &str = r#"
struct Cell { x: int }
mut cell := Cell { x: 0 }
let alias := cell
cell.x = 7
assert_equal(cell.x, 7)
assert_equal(alias.x, 0)
cell.x += 2
assert_equal(cell.x, 9)
func local() {
    mut c := Cell { x: 1 }
    c.x = 3
    assert_equal(c.x, 3)
    func update() { c.x += 4 return c.x }
    return update
}
let update := local()
assert_equal(update(), 7)
assert_equal(update(), 11)
let fixed := Cell { x: 5 }
mut caught := false
try { fixed.x = 10 } except err { caught = true }
assert(caught)
assert_equal(fixed.x, 5)
func immutable_capture() {
    const c := Cell { x: 2 }
    func change() {
        mut rejected := false
        try { c.x = 8 } except err { rejected = true }
        assert(rejected)
        return c.x
    }
    return change
}
let change := immutable_capture()
assert_equal(change(), 2)
"#;

#[test]
fn vm_field_assignment_writes_back() {
    run(FIELDS, true);
}

#[test]
fn interpreter_field_assignment_writes_back() {
    run(FIELDS, false);
}
