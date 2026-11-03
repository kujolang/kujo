// Diagnostic of outstanding Rust allocation sizes, NOT RSS or executable maps.
// Link unchanged source against separately frozen baseline/candidate libraries.
use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::interpreter::Value;
use kujo::vm::VM;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

static LIVE: AtomicUsize = AtomicUsize::new(0);
struct Accounting;
// SAFETY: forward every operation and its original layout to System. Accounting
// uses only atomics, never allocates, and changes only after allocation succeeds.
unsafe impl GlobalAlloc for Accounting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            if size >= layout.size() {
                LIVE.fetch_add(size - layout.size(), Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - size, Relaxed);
            }
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: Accounting = Accounting;

fn cycle(jit: bool, value: i64) -> usize {
    let mut chunk = BytecodeChunk::new();
    chunk.name = Some("live_allocation_probe".into());
    let constant = chunk.add_constant(Constant::Int(value));
    chunk.emit(OpCode::LoadConst(constant));
    chunk.emit(OpCode::Return);
    let function = Value::BytecodeFunction {
        chunk,
        captured: HashMap::new(),
        captured_binding_kinds: HashMap::new(),
    };
    let mut vm = VM::new();
    vm.set_jit_enabled(jit);
    assert_eq!(vm.jit_compile_bytecode_function(&function).unwrap(), jit);
    let result = vm.call_function_from_jit(function.clone(), vec![]).unwrap();
    assert!(matches!(result, Value::Int(actual) if actual == value));
    let live = LIVE.load(Relaxed);
    drop(result);
    drop(vm);
    drop(function);
    live
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let count: usize = args.get(1).expect("count").parse().unwrap();
    assert!((1..=64).contains(&count));
    let jit = match args.get(2).map(String::as_str) {
        Some("jit") => true,
        Some("vm") => false,
        _ => panic!("mode jit|vm"),
    };
    drop(args);
    // Exercise allocator accounting directly, including zeroing and both realloc
    // directions. No formatting or collection growth enters these observations.
    let before_control = LIVE.load(Relaxed);
    unsafe {
        let layout = Layout::from_size_align(4096, 16).unwrap();
        let pointer = ALLOCATOR.alloc_zeroed(layout);
        assert!(!pointer.is_null());
        assert_eq!(LIVE.load(Relaxed), before_control + 4096);
        let pointer = ALLOCATOR.realloc(pointer, layout, 8192);
        assert!(!pointer.is_null());
        assert_eq!(LIVE.load(Relaxed), before_control + 8192);
        let layout = Layout::from_size_align(8192, 16).unwrap();
        let pointer = ALLOCATOR.realloc(pointer, layout, 2048);
        assert!(!pointer.is_null());
        assert_eq!(LIVE.load(Relaxed), before_control + 2048);
        ALLOCATOR.dealloc(pointer, Layout::from_size_align(2048, 16).unwrap());
    }
    assert_eq!(LIVE.load(Relaxed), before_control);
    // Warm static runtime/compiler initialization once; this is a lifecycle
    // diagnostic, not an official benchmark or compilation-time measurement.
    cycle(jit, -1);
    let mut rows = Vec::with_capacity(count);
    let baseline = LIVE.load(Relaxed);
    for iteration in 0..count {
        let before = LIVE.load(Relaxed);
        let live = cycle(jit, iteration as i64);
        let after = LIVE.load(Relaxed);
        rows.push((iteration, before, live, after));
    }
    println!(
        "iteration,jit,baseline_live_bytes,before_live_bytes,vm_live_bytes,after_drop_live_bytes"
    );
    for (iteration, before, live, after) in rows {
        println!("{iteration},{jit},{baseline},{before},{live},{after}");
    }
}
