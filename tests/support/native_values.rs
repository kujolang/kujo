use kujo::builtins;
use kujo::interpreter::{Interpreter, Value};
use std::sync::Arc;

pub fn call(name: &str, args: &[Value]) -> Value {
    Interpreter::new().call_native_function_impl(name, args)
}

pub fn array(values: Vec<Value>) -> Value {
    Value::Array(Arc::new(values))
}

pub fn json(text: &str) -> Value {
    builtins::parse_json(text).unwrap()
}
