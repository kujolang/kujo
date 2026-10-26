#![cfg(feature = "runtime-jit")]

use kujo::interpreter::Value;
use kujo::jit::{
    jit_enter_function, jit_field_read, jit_indexed_read, jit_leave_function, jit_load_literal,
    VMContext,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::{collections::HashMap, sync::Arc};

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
fn record(bytes: usize) {
    let _ = TRACK.try_with(|enabled| {
        if enabled.get() {
            ALLOCATIONS.with(|count| count.set(count.get() + 1));
            REQUESTED_BYTES.with(|count| count.set(count.get() + bytes));
        }
    });
}

// SAFETY: all allocation operations delegate unchanged to System. Accounting
// uses allocation-free thread-local Cells and never dereferences allocation data.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: caller supplies a valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: caller supplies a valid allocation layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ownership and layout requirements are inherited from the caller.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        // SAFETY: pointer, original layout and new size are supplied by the caller.
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn fixed_dictionary_reads_do_not_clone_the_container() {
    for width in [8usize, 128, 1024] {
        for field in [false, true] {
            // SAFETY: all pointers refer to live exclusively borrowed context storage;
            // encoded roots stay live through the paired native activation.
            unsafe {
                let mut stack = Vec::new();
                let mut globals = HashMap::new();
                let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
                assert_eq!(jit_enter_function(&mut ctx), 0);
                let object = Value::FixedDict {
                    keys: Arc::new((0..width).map(|n| Arc::<str>::from(format!("k{n}"))).collect()),
                    values: vec![Value::Int(7); width].into(),
                };
                let target = jit_load_literal(&mut ctx, &object as *const Value as i64);
                let kind = ctx.return_value_kind;
                drop(object);
                let key = Value::Str(Arc::new("k0".into()));
                let index = jit_load_literal(&mut ctx, &key as *const Value as i64);
                let ikind = ctx.return_value_kind;
                let name = String::from("k0");
                ALLOCATIONS.with(|v| v.set(0));
                REQUESTED_BYTES.with(|v| v.set(0));
                TRACK.with(|v| v.set(true));
                for _ in 0..128 {
                    let got = if field {
                        jit_field_read(&mut ctx, target, kind, &name as *const String as i64)
                    } else {
                        jit_indexed_read(&mut ctx, target, kind, index, ikind)
                    };
                    assert_eq!(got, 7);
                    assert_eq!(ctx.return_value_kind, 0);
                }
                TRACK.with(|v| v.set(false));
                assert_eq!(ALLOCATIONS.with(Cell::get), 0, "width={width} field={field}");
                assert_eq!(REQUESTED_BYTES.with(Cell::get), 0);
                assert!(ctx.runtime_error.is_none());
                ctx.has_return_value = false;
                jit_leave_function(&mut ctx);
                assert!(ctx.runtime_error.is_none());
            }
        }
    }
}

use kujo::{
    bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode},
    jit::{invoke_compiled_fn, JitCompiler},
};
use std::hash::{Hash, Hasher};
#[test]
fn compiled_local_field_reads_do_not_copy_fixed_dictionary_storage() {
    for width in [8usize, 128, 1024] {
        for reads in [8usize, 128] {
            let mut compiler = JitCompiler::new().unwrap();
            let mut chunk = BytecodeChunk::new();
            chunk.params.push("item".into());
            chunk.local_names.push("item".into());
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
            chunk.local_count = 1;
            for _ in 0..reads {
                chunk.emit(OpCode::LoadLocal(0));
                chunk.emit(OpCode::FieldGet("k0".into()));
                chunk.emit(OpCode::Pop);
            }
            chunk.emit(OpCode::LoadLocal(0));
            chunk.emit(OpCode::FieldGet("k0".into()));
            chunk.emit(OpCode::Return);
            let compiled = compiler.compile_function(&chunk, "read_fields").unwrap();
            let value = Value::FixedDict {
                keys: Arc::new((0..width).map(|n| Arc::<str>::from(format!("k{n}"))).collect()),
                values: vec![Value::Int(7); width].into(),
            };
            let mut locals = HashMap::from([("item".into(), value.clone())]);
            let mut stack = Vec::new();
            let mut h = std::collections::hash_map::DefaultHasher::new();
            "item".hash(&mut h);
            let mut names = HashMap::from([(h.finish(), "item".into())]);
            let mut ctx = VMContext::new(&mut stack, &mut locals, std::ptr::null_mut());
            ctx.var_names_ptr = &mut names;
            assert_eq!(invoke_compiled_fn(compiled, &mut ctx), 0);
            assert_eq!(ctx.return_value, 7);
            locals.insert("item".into(), value);
            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            let status = invoke_compiled_fn(compiled, &mut ctx);
            TRACK.with(|v| v.set(false));
            assert_eq!(status, 0, "{:?}", ctx.runtime_error);
            assert_eq!(ctx.return_value, 7);
            assert_eq!(ctx.return_value_kind, 0);
            assert!(ctx.runtime_error.is_none());
            assert_eq!(ALLOCATIONS.with(Cell::get), 0, "width={width} reads={reads}");
            assert_eq!(REQUESTED_BYTES.with(Cell::get), 0);
        }
    }
}

#[test]
fn warmed_compiled_call_setup_has_bounded_allocations() {
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
            assert!(
                ALLOCATIONS.with(Cell::get) <= calls * 3,
                "arity={arity} calls={calls} allocations={} bytes={}",
                ALLOCATIONS.with(Cell::get),
                REQUESTED_BYTES.with(Cell::get)
            );
        }
    }
}

#[test]
fn vm_call_validation_does_not_allocate_per_parameter() {
    let mut counts = Vec::new();
    for arity in [1usize, 8] {
        let mut chunk = BytecodeChunk::new();
        chunk.name = Some("callee".into());
        for n in 0..arity {
            let name = format!("argument_{n}");
            chunk.params.push(name.clone());
            chunk.local_names.push(name);
            chunk.local_binding_kinds.push(BytecodeBindingKind::Mutable);
        }
        chunk.local_count = arity;
        chunk.emit(OpCode::LoadLocal(0));
        chunk.emit(OpCode::Return);
        let function = Value::BytecodeFunction {
            chunk,
            captured: HashMap::new(),
            captured_binding_kinds: HashMap::new(),
        };
        let mut vm = kujo::vm::VM::new();
        vm.set_jit_enabled(false);
        let inputs: Vec<_> =
            (0..128).map(|_| (function.clone(), vec![Value::Int(7); arity])).collect();
        ALLOCATIONS.with(|v| v.set(0));
        TRACK.with(|v| v.set(true));
        for (f, args) in inputs {
            assert!(matches!(vm.call_function_from_jit(f, args).unwrap(), Value::Int(7)));
        }
        TRACK.with(|v| v.set(false));
        counts.push(ALLOCATIONS.with(Cell::get));
    }
    assert_eq!(counts[0], counts[1], "parameter metadata allocation slope: {counts:?}");
}

#[test]
fn repeated_global_writes_do_not_copy_or_rescan_the_container() {
    use kujo::{bytecode::Constant, interpreter::Environment, vm::VM};
    use std::sync::Mutex;
    for (dictionary, named_write) in [(false, false), (true, false), (false, true), (true, true)] {
        let measure = |width: usize, writes: usize| {
            let mut chunk = BytecodeChunk::new();
            let value = chunk.add_constant(Constant::Int(7));
            let key = chunk.add_constant(if dictionary {
                Constant::String("k0".into())
            } else {
                Constant::Int(0)
            });
            for _ in 0..writes {
                chunk.emit(OpCode::LoadConst(value));
                chunk.emit(OpCode::LoadGlobal("data".into()));
                chunk.emit(OpCode::LoadConst(key));
                chunk.emit(OpCode::IndexSet);
                chunk.emit(if named_write {
                    OpCode::StoreVar("data".into())
                } else {
                    OpCode::StoreGlobal("data".into())
                });
                chunk.emit(OpCode::Pop);
            }
            if named_write {
                // Exercise real compiler output, not just a hand-built opcode pair.
                let key = if dictionary { "\"k0\"" } else { "0" };
                let source = format!("mut i := 0 while i < {writes} {{ data[{key}] = 7 i += 1 }}");
                let mut parser = kujo::parser::Parser::new(kujo::lexer::tokenize(&source).unwrap());
                chunk = kujo::compiler::Compiler::new().compile(&parser.parse()).unwrap();
            }
            let mut env = Environment::new();
            let root = if dictionary {
                Value::Dict(Arc::new(
                    (0..width).map(|i| (Arc::from(format!("k{i}")), Value::Int(0))).collect(),
                ))
            } else {
                Value::array(vec![Value::Int(0); width])
            };
            env.set("data".into(), root);
            let mut vm = VM::new();
            vm.set_globals(Arc::new(Mutex::new(env)));
            vm.set_jit_enabled(false);
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            let result = vm.execute(chunk);
            TRACK.with(|v| v.set(false));
            result.unwrap();
            REQUESTED_BYTES.with(Cell::get)
        };
        // Subtract first-write work, including initial depth validation. Only
        // repeated-mutation growth must be independent of container width.
        let small = measure(128, 64) - measure(128, 1);
        let large = measure(2048, 64) - measure(2048, 1);
        assert!(
            large <= small + 4096,
            "dictionary={dictionary} named_write={named_write} small={small} large={large}"
        );
    }
}

#[test]
fn environment_snapshots_do_not_duplicate_permission_names() {
    use kujo::interpreter::Environment;
    let mut environment = Environment::new();
    let width = 1024;
    for index in 0..width {
        environment.define(format!("binding_{index}"), Value::Int(index as i64));
    }
    ALLOCATIONS.with(|v| v.set(0));
    TRACK.with(|v| v.set(true));
    let snapshot = environment.clone();
    TRACK.with(|v| v.set(false));
    let count = ALLOCATIONS.with(Cell::get);
    // Value-map keys still need copying. A second per-binding allocation slope
    // means the immutable permission metadata is being copied again.
    assert!(count < width + 32, "snapshot allocations={count}, bindings={width}");
    assert!(matches!(snapshot.get("binding_1023"), Some(Value::Int(1023))));
}

#[test]
fn empty_local_scopes_do_not_allocate_permission_storage() {
    let mut environment = kujo::interpreter::Environment::new();
    // Warm the vectors once; subsequent scope creation/pop must not allocate.
    environment.push_scope();
    environment.pop_scope();
    ALLOCATIONS.with(|v| v.set(0));
    TRACK.with(|v| v.set(true));
    for _ in 0..128 {
        environment.push_scope();
        environment.pop_scope();
    }
    TRACK.with(|v| v.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
