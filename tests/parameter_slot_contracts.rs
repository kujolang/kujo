use kujo::{
    bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode},
    interpreter::Value,
    vm::VM,
};
use std::collections::HashMap;

fn callable(chunk: BytecodeChunk) -> Value {
    Value::BytecodeFunction {
        chunk,
        captured: HashMap::new(),
        captured_binding_kinds: HashMap::new(),
    }
}

#[test]
fn edited_parameter_order_does_not_reuse_snapshot_slot_mappings() {
    let mut chunk = BytecodeChunk::new();
    chunk.params = vec!["x".into(), "y".into()];
    chunk.local_names = vec!["x".into(), "y".into()];
    chunk.local_count = 2;
    chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable; 2];
    chunk.emit(OpCode::LoadLocal(0));
    chunk.emit(OpCode::Return);
    let original = callable(chunk.clone());
    let mut vm = VM::new();
    vm.set_jit_enabled(false);
    for _ in 0..3 {
        assert!(matches!(
            vm.call_function_from_jit(original.clone(), vec![Value::Int(3), Value::Int(7)])
                .unwrap(),
            Value::Int(3)
        ));
    }
    chunk.params.reverse();
    let edited = callable(chunk);
    assert!(matches!(
        vm.call_function_from_jit(edited, vec![Value::Int(3), Value::Int(7)]).unwrap(),
        Value::Int(7)
    ));
    assert!(matches!(
        vm.call_function_from_jit(original, vec![Value::Int(3), Value::Int(7)]).unwrap(),
        Value::Int(3)
    ));
}

#[test]
fn slotted_and_unslotted_legacy_named_parameters_keep_their_values() {
    for slotted in [false, true] {
        let mut chunk = BytecodeChunk::new();
        chunk.params.push("argument".into());
        if slotted {
            chunk.local_names.push("argument".into());
            chunk.local_count = 1;
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        }
        chunk.emit(OpCode::LoadVar("argument".into()));
        chunk.emit(OpCode::Return);
        let function = callable(chunk);
        let mut vm = VM::new();
        vm.set_jit_enabled(false);
        for value in [7, 9, 11] {
            assert!(matches!(
                vm.call_function_from_jit(function.clone(), vec![Value::Int(value)]).unwrap(),
                Value::Int(actual) if actual == value
            ));
        }
    }
}
