use kujo::compiler::Compiler;
use kujo::interpreter::Interpreter;
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};

fn run(source: &str, vm: bool) {
    run_with_jit(source, vm, false);
}

#[test]
fn cached_nested_writes_preserve_snapshots_and_external_mutations() {
    for vm in [false, true] {
        run(
            r#"
            struct State { rows: array }
            mut state := State { rows: [[0, 1]] }
            mut i := 0
            while i < 100 {
                let snapshot := state
                state.rows[0][-1] = i
                assert_equal(snapshot.rows[0][0], 0)
                if i > 0 { assert_equal(snapshot.rows[0][1], i - 1) }
                i += 1
            }
            let saved := state
            state.rows = [[2, 3], [4, 5]]
            state.rows[-1][-1] = 8
            assert_equal(saved.rows, [[0, 99]])
            assert_equal(state.rows, [[2, 3], [4, 8]])
            mut dict := {"rows": state.rows}
            let original := dict
            i = 0
            while i < 100 { dict["rows"][0][0] = i i += 1 }
            assert_equal(original["rows"], [[2, 3], [4, 8]])
            assert_equal(dict["rows"], [[99, 3], [4, 8]])
        "#,
            vm,
        );
    }
}

fn run_with_jit(source: &str, vm: bool, jit: bool) {
    let mut parser = Parser::new(tokenize(source).expect("tokenize"));
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut interpreter = Interpreter::new();
    if vm {
        let mut runtime = VM::new();
        runtime.set_jit_enabled(jit);
        runtime.set_globals(Arc::new(Mutex::new(interpreter.env)));
        runtime.execute(Compiler::new().compile(&parsed.stmts).expect("compile")).expect("VM");
    } else {
        interpreter.eval_stmts(&parsed.stmts);
        assert!(interpreter.return_value.is_none(), "{:?}", interpreter.return_value);
    }
}

const NESTED: &str = r#"
struct Cell { values: array }
mut matrix := [[0, 1], [2, 3]]
let original := matrix
matrix[0][1] = 7
matrix[-1][-1] += 4
assert_equal(matrix, [[0, 7], [2, 7]])
assert_equal(original, [[0, 1], [2, 3]])
mut cells := [Cell { values: [{"x": 1}] }]
cells[0].values[0]["x"] = 9
assert_equal(cells[0].values[0]["x"], 9)
mut state := {"rows": [[1]]}
state["rows"][0][0] += 2
assert_equal(state["rows"][0][0], 3)
func factory() {
    mut local := [[1]]
    local[0][0] = 4
    func update() { local[0][0] += 2 return local[0][0] }
    return update
}
let update := factory()
assert_equal(update(), 6)
assert_equal(update(), 8)
struct Box { cell: Cell }
mut boxed := Box { cell: Cell { values: [1] } }
boxed.cell.values = [2]
boxed.cell.values[0] *= 3
assert_equal(boxed.cell.values[0], 6)
mut keyed := {}
keyed[0] = 1
keyed[3] = null
mut wrapped := [keyed]
wrapped[0]["0"] = 5
wrapped[0]["00"] = 8
assert_equal(wrapped[0][0], 5)
assert_equal(wrapped[0]["00"], 8)
assert_equal(wrapped[0][3], null)
assert_equal(len(wrapped[0]), 3)
assert_equal(keyed[0], 1)
assert_equal(len(keyed), 2)
"#;

const ERRORS: &str = r#"
func fail() { throw("RHS failed") }
mut matrix := [[1]]
mut caught := false
try { matrix[0][9] = 2 } except err { caught = true }
assert(caught)
assert_equal(matrix, [[1]])
func immutable_local() {
    let rows := [[2]]
    mut rejected := false
    try { rows[0][0] = 3 } except err { rejected = true }
    assert(rejected)
    func change() {
        mut rejected := false
        try { rows[0][0] = 4 } except err { rejected = true }
        assert(rejected)
        return rows[0][0]
    }
    return change
}
let change := immutable_local()
assert_equal(change(), 2)
mut wrong := [1]
caught = false
try { wrong[0][0] = 2 } except err { caught = true }
assert(caught)
assert_equal(wrong, [1])
mut scalar := 9
caught = false
try { scalar = fail() } except err { caught = true }
assert(caught)
assert_equal(scalar, 9)
let fixed := [[2]]
caught = false
try { fixed[0][0] = 3 } except err { caught = true }
assert(caught)
assert_equal(fixed, [[2]])
const constant := {"rows": [[4]]}
caught = false
try { constant["rows"][0][0] = 5 } except err { caught = true }
assert(caught)
assert_equal(constant["rows"][0][0], 4)
mut data := {"present": [1]}
caught = false
try { data["missing"][0] = 5 } except err { caught = true }
assert(caught)
assert_equal(len(data), 1)
caught = false
try { matrix[0][0] = fail() } except err { caught = true }
assert(caught)
assert_equal(matrix, [[1]])
"#;

// A simple assignment evaluates its RHS first, then each target index once,
// from the root outwards. Updating the path must not evaluate indices again.
const ORDER: &str = r#"
mut order := ""
func rhs() { order += "r" return 7 }
func outer() { order += "o" return 0 }
func inner() { order += "i" return 0 }
mut rows := [[0]]
rows[outer()][inner()] = rhs()
assert_equal(order, "roi")
assert_equal(rows, [[7]])
func index_changes_root() { rows = [[3, 4]] return 0 }
rows[index_changes_root()][0] = 8
assert_equal(rows, [[8, 4]])
order = ""
func bad_index() { order += "b" throw("bad index") }
mut caught := false
try { rows[bad_index()][inner()] = rhs() } except err { caught = true }
assert(caught)
assert_equal(order, "rb")
assert_equal(rows, [[8, 4]])
"#;

#[test]
fn vm_nested_assignments() {
    run(NESTED, true);
}
#[test]
fn interpreter_nested_assignments() {
    run(NESTED, false);
}
#[test]
fn vm_nested_assignment_errors() {
    run(ERRORS, true);
}
#[test]
fn interpreter_nested_assignment_errors() {
    run(ERRORS, false);
}
#[test]
fn vm_nested_assignment_order() {
    run(ORDER, true);
}
#[test]
fn interpreter_nested_assignment_order() {
    run(ORDER, false);
}

#[test]
fn nested_assignment_falls_back_safely_with_jit_enabled() {
    run_with_jit(
        r#"
        func update() {
            mut rows := [[0]]
            mut i := 0
            while i < 1200 {
                rows[0][0] += 1
                i += 1
            }
            return rows[0][0]
        }
        assert_equal(update(), 1200)
        "#,
        true,
        true,
    );
}
