use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;

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

use kujo::bytecode::{BytecodeBindingKind, BytecodeChunk, OpCode};

fn main() {
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
