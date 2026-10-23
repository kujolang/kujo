use kujo::{
    bytecode::{BytecodeChunk, Constant, OpCode},
    interpreter::Value,
    vm::VM,
};
use std::collections::HashMap;

#[test]
fn sequential_cold_callbacks_do_not_accumulate_recursion_depth() {
    for explicit_return in [false, true] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some("cold".into());
        if explicit_return {
            let value = chunk.add_constant(Constant::Int(7));
            chunk.emit(OpCode::LoadConst(value));
            chunk.emit(OpCode::Return);
        } else {
            chunk.emit(OpCode::ReturnNone);
        }
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = VM::new();
        for call in 0..600 {
            let result =
                vm.call_function_from_jit(function.clone(), vec![]).unwrap_or_else(|error| {
                    panic!("explicit_return={explicit_return}, call={call}: {error}")
                });
            if explicit_return {
                assert!(matches!(result, Value::Int(7)));
            } else {
                assert!(matches!(result, Value::Null));
            }
        }
    }
}
