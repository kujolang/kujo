use kujo::{
    builtins,
    interpreter::{Interpreter, Value},
};
use std::{hint::black_box, sync::Arc, time::Instant};

fn main() {
    for n in [100, 1000, 5000] {
        let values = Value::Array(Arc::new(vec![Value::Str(Arc::new("kujo".into())); n]));
        let schema =
            builtins::parse_json(r#"{"items":{"type":"string","pattern":"^[a-z]+$"}}"#).unwrap();
        let args = [values, schema];
        let mut samples = Vec::new();
        let mut interpreter = Interpreter::new();
        for round in 0..7 {
            let start = Instant::now();
            let result =
                interpreter.call_native_function_impl("json_schema_validate", black_box(&args));
            let elapsed = start.elapsed().as_nanos();
            assert!(
                matches!(&result, Value::Dict(d) if matches!(d.get("valid"), Some(Value::Bool(true))))
            );
            black_box(result);
            if round >= 2 {
                samples.push(elapsed);
            }
        }
        samples.sort();
        println!("items={n} median_ns={} samples_ns={samples:?}", samples[2]);
    }
}
