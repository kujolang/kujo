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
