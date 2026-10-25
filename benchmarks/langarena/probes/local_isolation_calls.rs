use kujo::bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode};
use kujo::interpreter::Value;
use std::collections::HashMap;

fn call_controls() {
    for arity in [1usize, 4, 8] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some(format!("callback_{arity}"));
        for n in 0..arity {
            let name = format!("arg{n}");
            chunk.params.push(name.clone());
            chunk.local_names.push(name);
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        }
        chunk.local_count = arity;
        let index = chunk.add_constant(kujo::bytecode::Constant::Int(0));
        chunk.emit(OpCode::LoadConst(index));
        chunk.emit(OpCode::IndexGetInPlace(0));
        chunk.emit(OpCode::Return);
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = kujo::vm::VM::new();
        vm.set_jit_enabled(true);
        assert!(vm.jit_compile_bytecode_function(&function).unwrap());
        let arguments = || {
            let mut a = vec![Value::Int(0); arity];
            a[0] = Value::array(vec![Value::Int(7); 8]);
            a
        };
        assert!(matches!(
            vm.call_function_from_jit(function.clone(), arguments()).unwrap(),
            Value::Int(7)
        ));
        for calls in [1usize, 128] {
            let inputs: Vec<_> = (0..calls).map(|_| (function.clone(), arguments())).collect();
            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            for (f, a) in inputs {
                assert!(matches!(vm.call_function_from_jit(f, a).unwrap(), Value::Int(7)));
            }
            TRACK.with(|v| v.set(false));
            println!(
                "arity={arity} calls={calls} allocations={} bytes={}",
                ALLOCATIONS.with(Cell::get),
                REQUESTED_BYTES.with(Cell::get)
            );
        }
    }
}
