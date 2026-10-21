#[path = "support/native_api.rs"]
mod support;

#[test]
fn bytes_iterate_as_unsigned_integers_over_a_snapshot() {
    support::success(
        r#"
        mut data := bytes([0, 128, 255])
        mut seen := []
        for b in data { seen = push(seen, b) data[1] = 42 }
        assert_equal(seen, [0, 128, 255])
        assert_equal(data, bytes([0, 42, 255]))
        for b in bytes([]) { assert(false) }
        mut total := 0
        for b in bytes([1, 2, 3, 4]) {
            if b == 2 { continue }
            if b == 4 { break }
            total += b
        }
        assert_equal(total, 4)
        func first(data) { for b in data { return b } return -1 }
        assert_equal(first(bytes([255])), 255)
        mut caught := false
        try { for b in bytes([1]) { assert(false, "byte body") } }
        except err { caught = true }
        assert(caught)
    "#,
    );
}

#[test]
fn generator_byte_loops_preserve_snapshot_across_suspension() {
    support::success(
        r#"
        func* each(data) { for b in data { if b == 128 { continue } yield b } }
        mut source := bytes([0, 128, 255])
        let generator := each(source)
        source[2] = 1
        mut seen := []
        for b in generator { seen = push(seen, b) }
        assert_equal(seen, [0, 255])
        assert_equal(source[2], 1)
    "#,
    );
}
