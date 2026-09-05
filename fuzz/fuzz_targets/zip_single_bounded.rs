#![no_main]

mod binary_input;

use kujo::interpreter::{Interpreter, Value};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: i64 = 256 * 1024;

fuzz_target!(|data: &[u8]| {
    let input = binary_input::raw_or_hex(data, MAX_INPUT_BYTES);
    let mut interpreter = Interpreter::new();
    let result = interpreter.call_native_function_impl(
        "zip_single_file_read",
        &[Value::Bytes(input), Value::Int(MAX_OUTPUT_BYTES)],
    );
    assert!(matches!(result, Value::Dict(_) | Value::Error(_)));
});
