//! Compiler and ownership contracts for compatibility-preserving snapshot captures.
use kujo::bytecode::{BytecodeChunk, CaptureSource, Constant, OpCode};
use kujo::compiler::Compiler;
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};

fn compile(source: &str) -> BytecodeChunk {
    let mut parser = Parser::new(tokenize(source).unwrap());
    let parsed = parser.parse_with_diagnostics();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    Compiler::new().compile(&parsed.stmts).unwrap()
}

fn function<'a>(chunk: &'a BytecodeChunk, name: &str) -> &'a BytecodeChunk {
    chunk
        .constants
        .iter()
        .find_map(|constant| match constant {
            Constant::Function(child) if child.name.as_deref() == Some(name) => {
                Some(child.as_ref())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing function {name}"))
}

#[test]
fn captures_use_definition_site_slots_and_indexed_access() {
    let root = compile(
        r#"
        func factory() {
            mut value := 3
            let read := func() { value += 1; return value }
            if true { let value := 99 }
            return read
        }
    "#,
    );
    let factory = function(&root, "factory");
    let closure = function(factory, "<lambda>");
    assert_eq!(closure.upvalues, ["value"]);
    assert_eq!(closure.capture_sources, Some(vec![CaptureSource::Local(0)]));
    assert!(closure.instructions.contains(&OpCode::LoadCapture(0)));
    assert!(closure.instructions.contains(&OpCode::StoreCapture(0)));
    assert!(!closure.instructions.iter().any(|op| matches!(op,
        OpCode::LoadVar(name) | OpCode::StoreVar(name) if name == "value")));
}

#[test]
fn transitively_used_captures_are_forwarded_without_retaining_unused_locals() {
    let root = compile(
        r#"
        func factory() {
            let unused := [1, 2, 3]
            let value := 7
            return func() { return func() { return value } }
        }
    "#,
    );
    let middle = function(function(&root, "factory"), "<lambda>");
    let inner = function(middle, "<lambda>");
    assert_eq!(middle.upvalues, ["value"]);
    assert_eq!(middle.capture_sources, Some(vec![CaptureSource::Local(1)]));
    assert_eq!(inner.capture_sources, Some(vec![CaptureSource::Upvalue(0)]));
}

#[test]
fn local_shadowing_and_global_reads_do_not_capture_outer_values() {
    let root = compile(
        r#"
        let global_value := 8
        func factory() {
            let value := 7
            return func(value) { return value + global_value }
        }
    "#,
    );
    let closure = function(function(&root, "factory"), "<lambda>");
    assert!(closure.upvalues.is_empty());
    assert_eq!(closure.capture_sources, Some(vec![]));
}

#[test]
fn dropping_last_closure_releases_its_capture_cell() {
    let globals = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    vm.execute(compile(
        r#"
        func factory() { let value := [1, 2, 3]; return func() { return value } }
        let escaped := factory()
    "#,
    ))
    .unwrap();
    let escaped = globals.lock().unwrap().get("escaped").unwrap();
    let weak = match &escaped {
        Value::BytecodeFunction { captured, .. } => Arc::downgrade(&captured["value"]),
        _ => panic!("expected closure"),
    };
    drop(vm);
    drop(globals);
    assert!(weak.upgrade().is_some());
    drop(escaped);
    assert!(weak.upgrade().is_none(), "capture should not own its defining frame");
}

#[test]
fn malformed_capture_operands_return_errors_instead_of_panicking() {
    for opcode in [
        OpCode::LoadCapture(usize::MAX),
        OpCode::StoreCapture(usize::MAX),
        OpCode::MakeClosure(usize::MAX),
    ] {
        let mut chunk = BytecodeChunk::new();
        let constant = chunk.add_constant(Constant::None);
        chunk.emit(OpCode::LoadConst(constant));
        chunk.emit(opcode);
        assert!(VM::new().execute(chunk).is_err());
    }
}

#[test]
fn returned_named_recursive_closure_uses_frame_self_binding() {
    let globals = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    vm.execute(compile(
        r#"
        func factory(offset) {
            func fold(n) { if n == 0 { return offset }; return n + fold(n - 1) }
            return fold
        }
        let recursive := factory(7)
        let answer := recursive(3)
    "#,
    ))
    .unwrap();
    assert!(matches!(globals.lock().unwrap().get("answer"), Some(Value::Int(13))));
    assert!(globals.lock().unwrap().get("fold").is_none());
}

#[test]
#[cfg(feature = "runtime-jit")]
fn captured_calls_ignore_name_only_jit_cache_entries() {
    let globals = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    vm.execute(compile(
        r#"
        let plain := func() { return 100 }
        func factory() { mut n := 7; return func() { n += 1; return n } }
        let captured := factory()
    "#,
    ))
    .unwrap();
    let plain = globals.lock().unwrap().get("plain").unwrap();
    let captured = globals.lock().unwrap().get("captured").unwrap();
    vm.set_jit_enabled(true);
    assert!(vm.jit_compile_bytecode_function(&plain).unwrap());
    assert!(!vm.jit_compile_bytecode_function(&captured).unwrap());
    assert!(matches!(vm.call_function_from_jit(captured.clone(), vec![]).unwrap(), Value::Int(8)));
    assert!(matches!(vm.call_function_from_jit(captured, vec![]).unwrap(), Value::Int(9)));
}

#[test]
fn jit_to_vm_bridge_can_return_and_reinvoke_a_new_closure() {
    let globals = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    vm.execute(compile(
        r#"
        func factory(n) { return func() { n += 1; return n } }
    "#,
    ))
    .unwrap();
    let factory = globals.lock().unwrap().get("factory").unwrap();
    let closure = vm.call_function_from_jit(factory, vec![Value::Int(7)]).unwrap();
    assert!(matches!(vm.call_function_from_jit(closure.clone(), vec![]).unwrap(), Value::Int(8)));
    assert!(matches!(vm.call_function_from_jit(closure, vec![]).unwrap(), Value::Int(9)));
}

#[test]
fn vm_generator_retains_snapshot_across_yields_after_factory_return() {
    let globals = Arc::new(Mutex::new(Interpreter::new().env));
    let mut vm = VM::new();
    vm.set_globals(Arc::clone(&globals));
    vm.execute(compile(
        r#"
        func factory() {
            mut count := 4
            func* emit() { yield count; count += 1; yield count }
            return emit
        }
        let emit := factory()
        mut capture_total := 0
        for item in emit() { capture_total += item }
        let answer := capture_total
    "#,
    ))
    .unwrap();
    assert!(matches!(globals.lock().unwrap().get("answer"), Some(Value::Int(9))));
}
