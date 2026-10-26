use kujo::{
    bytecode::{BytecodeChunk, Constant, OpCode},
    vm::VM,
};

#[test]
fn swap_underflow_cannot_use_suspended_caller_operands() {
    for jit in [false, true] {
        for operands in [0, 1] {
            let mut root = BytecodeChunk::new();
            if operands == 1 {
                let constant = root.add_constant(Constant::Int(7));
                root.emit(OpCode::LoadConst(constant));
            }
            root.emit(OpCode::Swap);
            root.emit(OpCode::ReturnNone);
            let mut vm = VM::new();
            vm.set_jit_enabled(jit);
            let error = vm.execute(root).unwrap_err();
            assert!(error.contains("Stack underflow in Swap"), "{error}");
        }
        let mut callee = BytecodeChunk::new();
        callee.name = Some("invalid_swap".into());
        callee.emit(OpCode::Swap);
        callee.emit(OpCode::ReturnNone);
        let mut root = BytecodeChunk::new();
        // These operands belong to the caller and must not satisfy a callee's
        // malformed Swap, even though the physical stack contains two values.
        for value in [11, 22] {
            let constant = root.add_constant(Constant::Int(value));
            root.emit(OpCode::LoadConst(constant));
        }
        let function = root.add_constant(Constant::Function(Box::new(callee)));
        root.emit(OpCode::LoadConst(function));
        root.emit(OpCode::Call(0));
        root.emit(OpCode::ReturnNone);
        let mut vm = VM::new();
        vm.set_jit_enabled(jit);
        let error = vm.execute(root).unwrap_err();
        assert!(error.contains("Stack underflow in Swap"), "{error}");
    }
}
