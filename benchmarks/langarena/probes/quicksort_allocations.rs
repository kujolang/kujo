// Structural allocation probe, not a wall-time benchmark. Link against a built
// Kujo rlib (see ROOT_POOL_2026_10_24.md) and run from the repository root.
use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static LARGE: Cell<usize> = const { Cell::new(0) };
    static LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
fn record(bytes: usize) {
    let _ = TRACK.try_with(|enabled| {
        if enabled.get() {
            if bytes >= LIMIT.with(Cell::get) {
                LARGE.with(|n| n.set(n.get() + 1));
            }
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

use kujo::{compiler::Compiler, lexer, parser::Parser, vm::VM};
use std::sync::Arc;
fn main() {
    let enforce_reuse_budget = std::env::args().any(|arg| arg == "--expect-reuse");
    let port = std::fs::read_to_string("benchmarks/langarena/port.kujo").unwrap();
    let kernel = port
        .split("func quick_sort(input, low, high) {")
        .nth(1)
        .unwrap()
        .split("func merge_sort(")
        .next()
        .unwrap();
    let source=format!("func idiv(a,b) {{ return to_int(a / b) }}\nfunc quick_sort(input, low, high) {{{kernel}\nreturn quick_sort");
    for jit in [false, true] {
        let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
        let chunk = Compiler::new().compile(&parser.parse()).unwrap();
        let mut vm = VM::new();
        vm.set_jit_enabled(jit);
        let function = vm.execute(chunk).unwrap();
        let make = |n: usize| {
            let mut seed = 42u64;
            Arc::new(
                (0..n)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        Value::Int(((seed >> 8) % 1000000) as i64)
                    })
                    .collect::<Vec<_>>(),
            )
        };
        // Warm compilation and helper caches outside allocation accounting.
        vm.call_function_from_jit(
            function.clone(),
            vec![Value::Array(make(1024)), Value::Int(0), Value::Int(1023)],
        )
        .unwrap();
        for n in [128usize, 1024, 8192] {
            for shared in [false, true] {
                let values = make(n);
                let ptr = values.as_ptr();
                let snapshot = shared.then(|| values.clone());
                let mut expected: Vec<i64> = values
                    .iter()
                    .map(|v| if let Value::Int(i) = v { *i } else { panic!() })
                    .collect();
                expected.sort();
                let f = function.clone();
                let args = vec![Value::Array(values), Value::Int(0), Value::Int(n as i64 - 1)];
                ALLOCATIONS.with(|v| v.set(0));
                REQUESTED_BYTES.with(|v| v.set(0));
                LARGE.with(|v| v.set(0));
                LIMIT.with(|v| v.set(n * std::mem::size_of::<Value>()));
                TRACK.with(|v| v.set(true));
                let result = vm.call_function_from_jit(f, args);
                TRACK.with(|v| v.set(false));
                let Value::Array(sorted) = result.unwrap() else { panic!() };
                assert_eq!(ptr == sorted.as_ptr(), !shared, "unexpected array backing copy");
                assert_eq!(LARGE.with(Cell::get), usize::from(shared));
                if enforce_reuse_budget && jit && n == 8192 {
                    // Broad structural budgets: the pre-pool baseline exceeds both.
                    // Do not use machine-dependent elapsed time as a regression gate.
                    assert!(ALLOCATIONS.with(Cell::get) < 150_000);
                    assert!(REQUESTED_BYTES.with(Cell::get) < 35_000_000);
                }
                assert_eq!(sorted.len(), expected.len());
                assert!(sorted
                    .iter()
                    .zip(expected)
                    .all(|(v, e)| matches!(v,Value::Int(i) if *i==e)));
                if let Some(snapshot) = snapshot {
                    assert!(Value::equals(&Value::Array(snapshot), &Value::Array(make(n))));
                }
                println!("AUDIT jit={jit} n={n} shared={shared} allocations={} bytes={} array_sized_allocations={} same_backing={} value_size={}",ALLOCATIONS.with(Cell::get),REQUESTED_BYTES.with(Cell::get),LARGE.with(Cell::get),ptr==sorted.as_ptr(),std::mem::size_of::<Value>());
            }
        }
    }
}
