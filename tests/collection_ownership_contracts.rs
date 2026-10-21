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
                rows[0] = push(rows[0], i)
                i += 1
            }
            assert_equal(len(a), 50)
            assert_equal(a[-1], 52)
            assert_equal(rows[0][0], 49)
            a = fake(a, 9)
            assert_equal(a, [9])
            let p := pop(a)
            a = p[0]
            assert_equal(a, [])
            assert_equal(p[1], 9)
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
            interpreter.env.define(
                "host_struct".into(),
                Value::Struct {
                    name: "Host".into(),
                    fields: [
                        ("bad".into(), deep.clone()),
                        ("safe".into(), Value::array(vec![Value::Int(0)])),
                    ]
                    .into_iter()
                    .collect(),
                },
            );
            interpreter.env.define(
                "host_dict".into(),
                Value::Dict(Arc::new(
                    [
                        ("bad".into(), deep.clone()),
                        ("safe".into(), Value::array(vec![Value::Int(0)])),
                    ]
                    .into_iter()
                    .collect(),
                )),
            );
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
            try { host_struct.safe[0] = 1 } except e { failures += 1 }
            try { host_dict["safe"][0] = 1 } except e { failures += 1 }
            assert_equal(failures, 7)
            assert_equal(host_struct.safe[0], 0)
            assert_equal(host_dict["safe"][0], 0)
            host_struct.bad[0] = 0
            host_dict["bad"][0] = 0
            host_struct.safe[0] = 2
            host_dict["safe"][0] = 2
            assert_equal(host_struct.safe[0], 2)
            assert_equal(host_dict["safe"][0], 2)

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

#[test]
fn benchmark_state_writes_preserve_snapshots_and_failed_writes() {
    support::success(
        r#"
        struct State { rows: array }
        func work() {
            mut s := State { rows: [range(0, 2000)] }
            mut d := {"rows": [range(0, 2000)]}
            let before_s := s
            let before_d := d
            mut i := 0
            while i < 2000 {
                s.rows[0][i] = i + 1
                d["rows"][0][i] = i + 1
                i += 1
            }
            assert_equal(s.rows[0][-1], 2000)
            assert_equal(d["rows"][0][-1], 2000)
            assert_equal(before_s.rows[0][-1], 1999)
            assert_equal(before_d["rows"][0][-1], 1999)
            mut failures := 0
            try { s.rows[0][2000] = 9 } except e { failures += 1 }
            try { d["missing"][0] = 9 } except e { failures += 1 }
            assert_equal(failures, 2)
            assert_equal(s.rows[0][-1], 2000)
            assert_equal(d["rows"][0][-1], 2000)
            func update() { s.rows[0][0] = 99 d["rows"][0][0] = 88 return [s, d] }
            return update
        }
        let update := work()
        let first := update()
        assert_equal(first[0].rows[0][0], 99)
        assert_equal(first[1]["rows"][0][0], 88)
        let second := update()
        assert_equal(first, second)
    "#,
    );
    support::failure(
        "struct S { rows: array } let s := S { rows: [[0]] } s.rows[0][0] = 1",
        "immutable",
    );
    support::failure("const d := {\"rows\": [[0]]} d[\"rows\"][0][0] = 1", "const");
}

#[test]
fn field_appends_preserve_evaluation_order_aliases_and_fallbacks() {
    support::success(
        r#"
        struct State { items: array }
        mut s := State { items: [] }
        mut d := {"items": []}
        mut rows := [[]]
        mut i := 0
        while i < 3000 {
            s.items = push(s.items, i)
            d["items"] = append(d["items"], i)
            rows[0] = push(rows[0], i)
            i += 1
        }
        assert_equal(s.items, range(0,3000))
        assert_equal(d["items"], s.items)
        assert_equal(rows[0], s.items)
        let alias := s
        s.items = push(s.items, s.items)
        assert_equal(len(alias.items), 3000)
        assert_equal(s.items[-1], alias.items)
        mut log := ""
        func index() { log += "i" return 0 }
        func item() { log += "v" return 7 }
        rows[index()] = push(rows[index()], item())
        assert_equal(log, "ivi")
        assert_equal(rows[0][-1], 7)
        func replacement() { s.items = [99] return 8 }
        s.items = push(s.items, replacement())
        assert_equal(s.items[-1], 8)
        assert_equal(len(s.items), 3002)
        func fake(a,b) { return [b] }
        s.items = fake(s.items, 42)
        assert_equal(s.items, [42])
        let f := append
        s.items = f(s.items, 43)
        assert_equal(s.items, [42,43])
        mut failures := 0
        log = ""
        try { rows[index()] = push(3, item()) } except e { failures += 1 }
        assert_equal(log, "v")
        assert_equal(failures, 1)
        func fail() { throw("failed item") }
        try { s.items = push(s.items, fail()) } except e { failures += 1 }
        assert_equal(failures, 2)
        assert_equal(s.items, [42,43])
        mut switched := [[1], [2]]
        mut next := 0
        func moving_index() { let old := next next = 1 return old }
        switched[moving_index()] = push(switched[moving_index()], 3)
        assert_equal(switched, [[1], [1,3]])
        func factory() {
            mut state := State { items: [1] }
            func add() { state.items = push(state.items, 2) return state.items }
            state.items = push(state.items, 3)
            return add
        }
        let add := factory()
        assert_equal(add(), [1,2])
        assert_equal(add(), [1,2,2])
    "#,
    );
    support::failure(
        "struct S { items: array } let s := S { items: [] } s.items = push(s.items, 1)",
        "immutable",
    );
}

#[test]
fn lifo_updates_preserve_snapshots_order_and_empty_contracts() {
    support::success(
        r#"
        mut stack := Stack([1,2,3])
        let saved := stack
        stack = stack_push(stack, [4])
        let p := stack_pop(stack)
        mut item := p[1]
        item[0] = 99
        assert_equal(stack_to_array(saved), [1,2,3])
        assert_equal(stack_to_array(stack), [1,2,3,[4]])
        assert_equal(stack_to_array(p[0]), [1,2,3])
        assert_equal(stack_pop(Stack())[1], null)
        assert_equal(pop([])[1], 0)
        func run() {
            mut a := range(0, 10000)
            let original := a
            mut sum := 0
            while len(a) > 0 {
                let pair := pop(a)
                a = pair[0]
                sum += pair[1]
            }
            assert_equal(sum, 49995000)
            assert_equal(len(original), 10000)
            mut b := [[1],[2]]
            let snapshot := pop(b)
            b = snapshot[0]
            let second := pop(b)
            b = second[0]
            assert_equal(snapshot[0], [[1]])
            assert_equal(second[1], [1])
            assert_equal(b, [])
            mut empty := []
            let z := pop(empty)
            empty = z[0]
            assert_equal(z[1], 0)
            mut stack := Stack(range(0,10000))
            mut i := 9999
            while !stack_is_empty(stack) {
                let pair := stack_pop(stack)
                stack = pair[0]
                assert_equal(pair[1], i)
                i -= 1
            }
            assert_equal(i, -1)
        }
        run()
        func atomic_errors() {
            let fixed := [1,2]
            mut caught := false
            try { let rejected := pop(fixed) fixed = rejected[0] } except e { caught = true }
            assert(caught)
            assert_equal(fixed,[1,2])
            mut a := [1,2]
            let p := pop(a)
            assert_equal(a,[1,2])
            assert_equal(p[0],[1])
        }
        atomic_errors()
        func fake(a) { return [[9],8] }
        func shadow(pop) {
            mut a := [1,2]
            let p := pop(a)
            a = p[0]
            assert_equal(a,[9])
            assert_equal(p[1],8)
        }
        shadow(fake)
        func fail(a) { throw("pop failed") }
        mut a := [1]
        mut caught := false
        try { let p := fail(a) a = p[0] } except e { caught = true }
        assert(caught)
        assert_equal(a,[1])
        func closure() {
            mut a := [1,2,3]
            func next() { let p := pop(a) a = p[0] return p[1] }
            return next
        }
        let next := closure()
        assert_equal(next(),3)
        assert_equal(next(),2)
        assert_equal(next(),1)
        assert_equal(next(),0)
    "#,
    );
    support::failure("func f() { let a := [1] let p := pop(a) a = p[0] } f()", "immutable");
    support::failure("func f() { mut a := 1 let p := pop(a) a = p[0] } f()", "array");
    support::failure("mut a := [1] let p := 9 let p := pop(a) a = p[0]", "Duplicate");
}
