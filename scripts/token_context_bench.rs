pub use kujo::interpreter;
#[path = "../src/interpreter/native_functions/token.rs"]
mod token;
use interpreter::{DictMap, Value};
use std::{hint::black_box, sync::Arc, time::Instant};
fn message(role: &str, content: &str) -> Value {
    let mut map = DictMap::default();
    map.insert(Arc::from("role"), Value::Str(Arc::new(role.into())));
    map.insert(Arc::from("content"), Value::Str(Arc::new(content.into())));
    Value::Dict(Arc::new(map))
}
fn main() {
    for n in [100, 1000, 5000] {
        let mut messages = vec![message("system", "Keep evidence concise.")];
        for _ in 0..n {
            messages.push(message(
                "user",
                "Context covering security, replay, schema validation, and product posture.",
            ));
        }
        let args = [Value::Array(Arc::new(messages)), Value::Int(64)];
        let mut samples = Vec::new();
        for round in 0..9 {
            let start = Instant::now();
            let result = token::handle("ai_fit_context", black_box(&args)).unwrap();
            let elapsed = start.elapsed().as_nanos();
            if round > 1 {
                samples.push(elapsed);
            }
            if round == 0 {
                println!("n={n} result={result:?}");
            }
            black_box(result);
        }
        samples.sort();
        println!("n={n} median_ns={} samples_ns={samples:?}", samples[samples.len() / 2]);
    }
}
