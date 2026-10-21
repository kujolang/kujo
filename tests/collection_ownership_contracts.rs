#[path = "support/native_api.rs"]
mod support;

#[test]
fn queue_updates_preserve_snapshots_order_and_nested_values() {
    support::success(
        r#"
        mut q := Queue([1, 2, 3])
        let original := q
        q = queue_enqueue(q, 4)
        assert_equal(queue_to_array(original), [1, 2, 3])
        assert_equal(queue_to_array(q), [1, 2, 3, 4])
        let first := queue_dequeue(q)
        let branch := queue_dequeue(q)
        assert_equal(first[1], 1)
        assert_equal(queue_to_array(first[0]), [2, 3, 4])
        assert_equal(queue_to_array(branch[0]), [2, 3, 4])
        assert_equal(queue_to_array(q), [1, 2, 3, 4])
        mut nested := Queue([[1], [2]])
        let before := nested
        let pair := queue_dequeue(nested)
        mut item := pair[1]
        item[0] = 99
        assert_equal(queue_to_array(before), [[1], [2]])
        assert_equal(queue_to_array(pair[0]), [[2]])
        func factory() {
            mut q := Queue([1])
            func add() { q = queue_enqueue(q, 2) return q }
            q = queue_enqueue(q, 3)
            assert_equal(queue_to_array(q), [1, 3])
            return add
        }
        let add := factory()
        assert_equal(queue_to_array(add()), [1, 2])
        assert_equal(queue_to_array(add()), [1, 2, 2])
        mut cycle := Queue(range(0, 1000))
        mut i := 0
        while i < 10000 {
            let pair := queue_dequeue(cycle)
            assert_equal(pair[1], i)
            cycle = queue_enqueue(pair[0], i + 1000)
            i += 1
        }
        assert_equal(queue_size(cycle), 1000)
        assert_equal(queue_peek(cycle), 10000)
        assert_equal(queue_to_array(cycle), range(10000, 11000))
        assert_equal(queue_dequeue(Queue())[1], null)
        assert(queue_is_empty(queue_dequeue(Queue([1]))[0]))
    "#,
    );
}

#[test]
fn nested_writes_keep_error_atomicity_and_expression_order() {
    support::success(
        r#"
        mut rows := [[1, 2], [3, 4]]
        let original := rows
        rows[0][1] = 9
        assert_equal(original, [[1, 2], [3, 4]])
        assert_equal(rows, [[1, 9], [3, 4]])
        mut count := 0
        func index() { count += 1 return 0 }
        func rhs() { count += 10 return 7 }
        rows[index()][index()] = rhs()
        assert_equal(count, 12)
        assert_equal(rows[0][0], 7)
        let saved := rows
        mut caught := false
        try { rows[0][999] = 8 } except e { caught = true }
        assert(caught)
        assert_equal(rows, saved)
    "#,
    );
    support::failure("let rows := [[1]]\nrows[0][0] = 2", "immutable");
}

#[test]
fn append_rebinding_preserves_aliases_callbacks_shadowing_and_failures() {
    support::success(
        r#"
        mut a := [1]
        let saved := a
        a = push(a, 2)
        assert_equal(saved, [1])
        assert_equal(a, [1, 2])
        let f := append
        a = f(a, 3)
        assert_equal(a, [1, 2, 3])
        a = push(a, a)
        assert_equal(a, [1, 2, 3, [1, 2, 3]])
        func fake(a, item) { return [item] }
        func shadow(push) {
            mut a := [1]
            a = push(a, 7)
            assert_equal(a, [7])
        }
        shadow(fake)
        mut b := [1]
        func change() { b = [9] return 2 }
        b = push(b, change())
        assert_equal(b, [1, 2])
        func fail() { throw("item failed") }
        let old := b
        mut caught := false
        try { b = push(b, fail()) } except e { caught = true }
        assert(caught)
        assert_equal(b, old)
        func factory() {
            mut a := [1]
            func add() { a = push(a, 2) return a }
            a = push(a, 3)
            assert_equal(a, [1, 3])
            return add
        }
        let add := factory()
        assert_equal(add(), [1, 2])
        assert_equal(add(), [1, 2, 2])
        func local() {
            mut a := []
            mut i := 0
            while i < 10000 { a = push(a, i * 2) i += 1 }
            assert_equal(a[-1], 19998)
        }
        local()
    "#,
    );
    support::failure("let a := [1]\na = push(a, 2)", "immutable");
    support::failure("mut a := 1\na = push(a, 2)", "array");
}

#[test]
fn append_keeps_each_runtimes_callable_resolution_order() {
    let source = r#"
        func fake(a, item) { return [item] }
        mut selected := push
        func change() { selected = fake return 2 }
        mut a := [1]
        a = selected(a, change())
        print(to_json(a))
    "#;
    for interpreter in [false, true] {
        let out = support::run(source, interpreter);
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let expected = if interpreter { "[1,2]" } else { "[2]" };
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), expected);
    }
}

#[test]
fn collection_fast_paths_survive_optimizer_relocation_and_jit_fallback() {
    use kujo::{
        compiler::Compiler, interpreter::Interpreter, lexer::tokenize, optimizer::Optimizer,
        parser::Parser, vm::VM,
    };
    use std::sync::{Arc, Mutex};
    let source = r#"
        func fake(a, item) { return [item] }
        func run() {
            mut a := []
            mut rows := [[0]]
            if false { print("unreachable") }
            mut i := 0
            while i < 50 {
                a = push(a, i + (1 + 2))
                rows[0][0] = i
                i += 1
            }
            assert_equal(len(a), 50)
            assert_equal(a[-1], 52)
            assert_equal(rows[0][0], 49)
            a = fake(a, 9)
            assert_equal(a, [9])
        }
        run()
    "#;
    for optimize in [false, true] {
        for jit in [false, true] {
            let mut parser = Parser::new(tokenize(source).unwrap());
            let parsed = parser.parse_with_diagnostics();
            assert!(parsed.diagnostics.is_empty());
            let mut chunk = Compiler::new().compile(&parsed.stmts).unwrap();
            if optimize {
                Optimizer::new().optimize(&mut chunk);
            }
            let mut vm = VM::new();
            vm.set_globals(Arc::new(Mutex::new(Interpreter::new().env)));
            vm.set_jit_enabled(jit);
            vm.execute(chunk).unwrap();
        }
    }
}

#[test]
fn collection_fast_paths_preserve_existing_vm_depth_checks() {
    // A host-supplied deep value isolates depth enforcement from construction.
    // The larger stack is only for dropping this deliberately deep test fixture.
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            use kujo::{
                compiler::Compiler,
                interpreter::{Interpreter, Value},
                lexer::tokenize,
                parser::Parser,
                vm::VM,
            };
            use std::sync::{Arc, Mutex};
            let mut deep = Value::Int(0);
            for _ in 0..kujo::runtime_limits::DEFAULT_MAX_VALUE_NESTING_DEPTH {
                deep = Value::array(vec![deep]);
            }
            let mut interpreter = Interpreter::new();
            interpreter.env.define("deep".into(), deep);
            let source = r#"
            mut items := []
            mut rows := [[0]]
            mut q := Queue()
            mut failures := 0
            // Native results historically do not impose the bytecode builder
            // depth limit. Preserve that behavior via the ordinary-call fallback.
            items = push(items, deep)
            q = queue_enqueue(q, deep)
            try { let wrapped := [items] } except e { failures += 1 }
            try { rows[0][0] = deep } except e { failures += 1 }
            try { let wrapped := [q] } except e { failures += 1 }
            assert_equal(failures, 3)
            assert_equal(len(items), 1)
            assert_equal(rows, [[0]])
            assert_equal(queue_size(q), 1)
            mut mixed := push(items, [0])
            try { mixed[1][0] = 8 } except e { failures += 1 }
            assert_equal(failures, 4)
            assert_equal(mixed[1][0], 0)
            mixed[0][0] = 0
            assert_equal(mixed[0], [0])
            // A normal array write must invalidate prior depth-validation identity.
            rows[0][0] = 1
            rows = push(rows, deep)
            try { rows[0][0] = 8 } except e { failures += 1 }
            assert_equal(failures, 5)
            assert_equal(rows[0][0], 1)
            rows[1][0] = 0
            assert_equal(rows[1], [0])
        "#;
            let mut parser = Parser::new(tokenize(source).unwrap());
            let parsed = parser.parse_with_diagnostics();
            assert!(parsed.diagnostics.is_empty());
            let mut vm = VM::new();
            vm.set_globals(Arc::new(Mutex::new(interpreter.env)));
            vm.execute(Compiler::new().compile(&parsed.stmts).unwrap()).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}
