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
