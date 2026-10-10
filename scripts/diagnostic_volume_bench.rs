//! Reproducible native diagnostic volume receipt (no provider/tokenizer calls).
use kujo::interpreter::{Interpreter, Value};
use std::sync::Arc;

fn main() {
    let mut runtime = Interpreter::new();
    for name in ["parse_int", "parse_float", "to_int", "to_float", "parse_toml"] {
        let input = if name == "parse_toml" {
            format!("a = {}!", "x".repeat(8192))
        } else {
            "x".repeat(8192)
        };
        let Value::Error(message) =
            runtime.call_native_function_impl(name, &[Value::Str(Arc::new(input.clone()))])
        else {
            panic!("expected rejected input")
        };
        let bytes = message.len();
        let Value::Int(estimate) =
            runtime.call_native_function_impl("ai_count_tokens", &[Value::Str(Arc::new(message))])
        else {
            panic!("expected deterministic token estimate")
        };
        println!(
            "surface={name} input_bytes={} error_bytes={bytes} estimated_tokens={estimate}",
            input.len()
        );
    }
}
