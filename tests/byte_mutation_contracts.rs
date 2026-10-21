#[path = "support/native_api.rs"]
mod support;
use support::{failure, success};

#[test]
fn utf8_conversion_is_explicit_and_strict() {
    success(
        r#"
        let text := "Kujo ☃ café"
        let b := utf8_encode(text)
        assert_equal(utf8_decode(b), text)
        assert_equal(byte_length(b), byte_length(text))
        assert_equal(utf8_encode(""), bytes([]))
        assert_equal(utf8_decode(bytes([])), "")
        assert_equal(utf8_encode("é"), bytes([195, 169]))
        assert_equal(utf8_encode(utf8_decode(bytes([0, 65]))), bytes([0, 65]))
        let encode := utf8_encode
        assert_equal(encode("A"), bytes([65]))
    "#,
    );
    for source in [
        "utf8_decode(bytes([255]))",
        "utf8_decode(bytes([195]))",
        "utf8_decode(bytes([192, 128]))",
        "utf8_decode(bytes([237, 160, 128]))",
    ] {
        failure(source, "invalid UTF-8");
    }
    failure("utf8_decode(\"a\")", "bytes");
    failure("utf8_encode(bytes([65]))", "string");
    failure("utf8_encode()", "argument");
    failure("utf8_decode(bytes([]), 1)", "argument");
}

#[test]
fn byte_writes_preserve_copies_at_every_assignment_scope() {
    success(
        r#"
        mut b := bytes([0, 127, 255])
        let old := b
        b[0] = 65
        b[-1] -= 1
        assert_equal(b, bytes([65, 127, 254]))
        assert_equal(old, bytes([0, 127, 255]))
        func local() {
            mut data := bytes([1, 2])
            let copy := data
            data[0] = 3
            data[-1] += 1
            assert_equal(copy, bytes([1, 2]))
            func change() { data[0] += 1 return data }
            assert_equal(change(), bytes([4, 3]))
            assert_equal(data, bytes([3, 3]))
            return change
        }
        let change := local()
        assert_equal(change(), bytes([5, 3]))
        mut rows := [bytes([1, 2])]
        let copy := rows
        rows[0][-1] = 255
        assert_equal(copy[0], bytes([1, 2]))
        assert_equal(rows[0], bytes([1, 255]))
        struct Box { data: any }
        mut box := Box { data: bytes([3]) }
        box.data[0] = 8
        assert_equal(box.data[0], 8)
    "#,
    );
}

#[test]
fn invalid_byte_writes_are_atomic_and_catchable() {
    success(
        r#"
        mut b := bytes([1])
        let copy := b
        mut caught := 0
        try { b[0] = 256 } except e { caught += 1 }
        try { b[-2] = 2 } except e { caught += 1 }
        try { b[1] = 2 } except e { caught += 1 }
        try { b[0.5] = 2 } except e { caught += 1 }
        try { b[0] = -1 } except e { caught += 1 }
        try { b[0] = 1.5 } except e { caught += 1 }
        assert_equal(caught, 6)
        assert_equal(b, copy)
        func local() {
            mut b := bytes([2])
            mut caught := false
            try { b[0] = 999 } except e { caught = true }
            assert(caught)
            assert_equal(b, bytes([2]))
        }
        local()
        mut rows := [bytes([3])]
        try { rows[0][0] = 256 } except e { caught += 1 }
        assert_equal(rows[0], bytes([3]))
        assert_equal(caught, 7)
    "#,
    );
    failure("let b := bytes([1])\nb[0] = 2", "immutable");
}
