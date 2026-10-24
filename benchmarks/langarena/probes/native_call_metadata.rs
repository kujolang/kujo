//! Diagnostic: executed native work is constant; only unreachable bytecode grows.
use kujo::bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode};
use kujo::interpreter::Value;
use kujo::vm::VM;
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    for padding in [0, 1_000, 10_000] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some(format!("metadata_{padding}"));
        chunk.params = vec!["a".into(), "b".into()];
        chunk.local_names = chunk.params.clone();
        chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable; 2];
        chunk.local_count = 2;
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::Return);
        chunk.instructions.extend(std::iter::repeat_n(OpCode::Nop, padding));
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = VM::new();
        vm.set_jit_enabled(true);
        assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        for round in 0..4 {
            let start = Instant::now();
            for _ in 0..10_000 {
                let result = vm
                    .call_function_from_jit(function.clone(), vec![Value::Int(7), Value::Int(9)])
                    .unwrap();
                assert!(matches!(result, Value::Int(7)));
            }
            println!(
                "padding={padding} round={round} calls=10000 ns={}",
                start.elapsed().as_nanos()
            );
        }
    }
}
