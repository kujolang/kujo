#[path = "support/native_api.rs"]
mod support;

#[test]
fn dictionary_deletion_preserves_pairs_aliases_and_key_contracts() {
    support::success(
        r#"
        func work() {
            mut m := {} m["a"] = [1] m["b"] = null m["0"] = 7
            let old := m
            let snapshot := func() { return m }
            let p := remove(m, "a")
            m = p[0]
            assert_equal(p[1], [1])
            assert_equal(old["a"], [1])
            assert_equal(snapshot()["a"], [1])
            let q := remove(m, "b")
            m = q[0]
            assert_equal(q[1], null)
            let r := remove(m, 0)
            m = r[0]
            assert_equal(r[1], 7)
            let missing := remove(m, "absent")
            m = missing[0]
            assert_equal(missing[1], 0)
            assert_equal(len(m), 0)
            assert_equal(len(p[0]), 2)
            mut dense := {} dense[0] = null dense[1] = 8 dense[2] = 9
            let d := remove(dense, 0)
            dense = d[0]
            assert_equal(d[1], null)
            let e := remove(dense, "1")
            dense = e[0]
            assert_equal(e[1], 8)
            let f := remove(dense, "02")
            dense = f[0]
            assert_equal(f[1], 0)
            assert_equal(dense[2], 9)
        }
        work()
    "#,
    );
}

#[test]
fn removal_fallbacks_keep_callable_effects_and_immutable_errors() {
    support::success(
        r#"
        func work() {
            mut m := {"a": 1}
            func key() { m = {"b": 2} return "a" }
            let p := remove(m, key())
            m = p[0]
            assert_equal(len(m), 0)
            let custom := func(a, k) { return [{"z": 3}, k] }
            let q := custom(m, "x")
            m = q[0]
            assert_equal(m["z"], 3)
            assert_equal(q[1], "x")
            let frozen := {"x": 1}
            mut caught := false
            try {
                let r := remove(frozen, "x")
                frozen = r[0]
            } except err { caught = true }
            assert(caught)
            assert_equal(frozen["x"], 1)
            mut array := [1, 2, 1]
            let removed := remove(array, 1)
            assert_equal(removed, [2, 1])
        }
        work()
    "#,
    );
}

#[test]
fn repeated_removal_keeps_retained_pairs_and_callable_aliases() {
    support::success(
        r#"
        func work() {
            let erase := remove
            mut m := {} mut i := 0
            while i < 128 { m["k" + str(i)] = i i += 1 }
            mut saved := []
            i = 0
            while i < 128 {
                let p := erase(m, "k" + str(i))
                m = p[0]
                if i == 0 { saved = push(saved, p) }
                assert_equal(p[1], i)
                i += 1
            }
            assert_equal(len(m), 0)
            assert_equal(len(saved[0][0]), 127)
            assert_equal(saved[0][0]["k127"], 127)
            m["safe"] = 1
            func fail() { throw("key failed") }
            mut caught := false
            try { let p := erase(m, fail()) m = p[0] } except err { caught = true }
            assert(caught)
            assert_equal(m["safe"], 1)
        }
        work()
    "#,
    );
}

#[test]
fn field_and_indexed_pops_preserve_snapshots_and_removed_values() {
    support::success(
        r#"
        struct State { rows: array }
        func work() {
            mut state := State { rows: [[1, 2, 3], [4, 5]] }
            let alias := state
            let snapshot := func() { return state }
            let i := 0
            let p := pop(state.rows[i])
            state.rows[i] = p[0]
            assert_equal(p[1], 3)
            assert_equal(state.rows[0], [1, 2])
            assert_equal(alias.rows[0], [1, 2, 3])
            assert_equal(snapshot().rows[0], [1, 2, 3])
            let q := pop(state.rows[i])
            state.rows[i] = q[0]
            assert_equal(q[1], 2)
            assert_equal(p[0], [1, 2])
            mut nested := {"rows": [[7, 8]]}
            let old := nested
            let r := pop(nested["rows"][0])
            nested["rows"][0] = r[0]
            assert_equal(nested["rows"][0], [7])
            assert_equal(old["rows"][0], [7, 8])
            mut rows := [[10, 11], []]
            let last := pop(rows[-1])
            rows[-1] = last[0]
            assert_equal(last[1], 0)
            let first := pop(rows[0])
            rows[0] = first[0]
            assert_equal(first[1], 11)
        }
        work()
    "#,
    );
}

#[test]
fn indexed_pop_fallbacks_preserve_index_effects_and_shadowing() {
    support::success(
        r#"
        func work() {
            mut rows := [[1, 2], [3, 4]]
            mut calls := 0
            func index() { let old := calls calls += 1 return old }
            let p := pop(rows[index()])
            rows[index()] = p[0]
            assert_equal(index(), 2)
            assert_equal(rows, [[1, 2], [1]])
            let custom := func(a) { return [[99], 88] }
            let q := custom(rows[0])
            rows[0] = q[0]
            assert_equal(rows[0], [99])
            assert_equal(q[1], 88)
        }
        work()
    "#,
    );
    support::failure(
        "func work() { let frozen := [[1,2]] let p := pop(frozen[0]) frozen[0] = p[0] } work()",
        "immutable",
    );
}

#[test]
fn nested_map_removal_rechecks_destination_after_key_effects() {
    support::success(
        r#"
        mut rows := [{"a": 1}, {"b": 2}]
        mut index := 0
        func key() { index = 1 return "a" }
        func work() {
            let p := remove(rows[index], key())
            rows[index] = p[0]
            assert_equal(rows, [{"a": 1}, {}])
            mut state := {"map": {}}
            state["map"]["x"] = 9
            let alias := state
            let q := remove(state["map"], "x")
            state["map"] = q[0]
            assert_equal(q[1], 9)
            assert_equal(len(state["map"]), 0)
            assert_equal(alias["map"]["x"], 9)
        }
        work()
    "#,
    );
}

#[test]
fn integer_keyed_parents_keep_indexed_pop_value_semantics() {
    support::success(
        r#"
        func work() {
            mut dense := {} dense[0] = [1, 2, 3]
            let old := dense
            let p := pop(dense[0])
            dense[0] = p[0]
            assert_equal(dense[0], [1, 2])
            assert_equal(old[0], [1, 2, 3])
            mut sparse := {} sparse[-1] = [7, 8]
            let q := pop(sparse[-1])
            sparse[-1] = q[0]
            assert_equal(q[1], 8)
            assert_equal(sparse[-1], [7])
        }
        work()
    "#,
    );
}

#[test]
fn failed_nested_pop_writeback_keeps_overdeep_host_state_unchanged() {
    std::thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(|| {
        use kujo::{compiler::Compiler, interpreter::{Interpreter, Value}, lexer::tokenize, parser::Parser, vm::VM};
        use std::sync::{Arc, Mutex};
        let mut deep = Value::Int(0);
        for _ in 0..kujo::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH { deep = Value::array(vec![deep]); }
        let mut interpreter = Interpreter::new();
        interpreter.env.define("host".into(), Value::dict([
            ("bad".into(),deep),
            ("safe".into(),Value::array(vec![Value::array(vec![Value::Int(1),Value::Int(2)])])),
        ].into_iter().collect()));
        let source = r#"
            func work() {
                mut caught := false
                try { let p := pop(host["safe"][0]) host["safe"][0] = p[0] } except err { caught = true }
                assert(caught)
                assert_equal(host["safe"][0], [1,2])
            }
            work()
        "#;
        let parsed = Parser::new(tokenize(source).unwrap()).parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        let mut vm = VM::new();
        vm.set_globals(Arc::new(Mutex::new(interpreter.env)));
        vm.execute(Compiler::new().compile(&parsed.stmts).unwrap()).unwrap();
    }).unwrap().join().unwrap();
}
