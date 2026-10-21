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
