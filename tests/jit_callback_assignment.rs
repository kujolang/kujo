use kujo::{
    bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode},
    interpreter::Value,
    vm::VM,
};
use std::{collections::HashMap, process::Command};

#[test]
fn callback_bytecode_stores_leave_the_assignment_value_for_pop() {
    for (store, load) in [
        (OpCode::StoreVar("value".into()), OpCode::LoadVar("value".into())),
        (OpCode::StoreLocal(0), OpCode::LoadLocal(0)),
        (OpCode::StoreGlobal("value".into()), OpCode::LoadGlobal("value".into())),
        (
            OpCode::DefineGlobal("value".into(), BytecodeBindingKind::Mutable),
            OpCode::LoadGlobal("value".into()),
        ),
    ] {
        let mut chunk = BytecodeChunk::new();
        chunk.local_names = vec!["slot".into()];
        chunk.local_binding_kinds = vec![BytecodeBindingKind::Mutable];
        let value = chunk.add_constant(Constant::Int(7));
        for op in [OpCode::LoadConst(value), store.clone(), OpCode::Pop, load, OpCode::Return] {
            chunk.emit(op);
        }
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = VM::new();
        let result = vm.call_function_from_jit(function, vec![]);
        assert!(matches!(result, Ok(Value::Int(7))), "{store:?}: {result:?}");
    }
}

#[test]
fn compiled_caller_can_update_bindings_through_a_cold_bytecode_callee() {
    let source = r#"
        mut before := 0
        mut after := 0
        func calculate(a, b) {
            before += 1
            mut result := a + b
            result = result + 1
            after += 1
            return result
        }
        func relay(a, b) { return calculate(a, b) }
        mut i := 0
        while i < 200 { assert_equal(relay(6, 2), 9) i += 1 }
        assert_equal(before, 200)
        assert_equal(after, 200)
        print("PASS")
    "#;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("cold_callee.kujo");
    std::fs::write(&file, source).unwrap();
    for mode in [None, Some("--interpreter"), Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]).arg(&file);
        if let Some(flag) = mode {
            command.arg(flag);
        }
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1");
        }
        let output = command.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            assert!(stderr.contains("Successfully compiled function 'relay'"), "{stderr}");
            assert!(
                !stderr.contains("Successfully compiled function 'calculate'"),
                "callee must remain cold: {stderr}"
            );
        }
    }
}
