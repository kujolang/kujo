use kujo::interpreter::Value;
use std::sync::Arc;

#[path = "support/native_api.rs"]
mod support;

#[test]
fn cloning_bytes_shares_storage_independent_of_buffer_size() {
    for size in [0, 1, 1024, 1_000_000] {
        let original = Value::bytes(vec![255; size]);
        let cloned = original.clone();
        let (Value::Bytes(a), Value::Bytes(b)) = (&original, &cloned) else { unreachable!() };
        assert!(Arc::ptr_eq(a, b), "cloning {size} bytes copied the buffer");
        assert_eq!(a.as_slice(), b.as_slice());
    }
}

#[test]
fn byte_reads_slices_and_binary_roundtrips_work_in_both_runtimes() {
    support::success(
        r#"
        let b := bytes([0, 127, 128, 255])
        assert_equal(len(b), 4)
        assert_equal(byte_length(b), 4)
        assert_equal(b[0], 0)
        assert_equal(b[-1], 255)
        assert_equal(slice(b, 1, 3), bytes([127, 128]))
        assert_equal(concat(slice(b, 0, 2), slice(b, 2, 4)), b)
        assert_equal(decode_base64(encode_base64(b)), b)
        assert_equal(bytes_is_ascii(b), false)
        assert_equal(bytes_is_ascii(bytes([])), true)
    "#,
    );
    support::failure("bytes([256])", "range 0-255");
    support::failure("bytes([-1])", "range 0-255");
    support::failure("bytes([1.5])", "integers");
    support::failure("let b := bytes([1])\nprint(b[1])", "bounds");
}
