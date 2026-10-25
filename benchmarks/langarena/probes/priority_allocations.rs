// Structural allocation probe, not a wall-time benchmark. Link against a built
// Kujo rlib (see ROOT_POOL_2026_10_24.md) and run from the repository root.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static LARGE: Cell<usize> = const { Cell::new(0) };
    static LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
    // Fixed TLS accounting does not allocate inside the allocator callback.
    // Bucket 2048 combines requests >= 2048 bytes; other buckets are exact sizes.
    static SIZES: [Cell<usize>; 2049] = const { [const { Cell::new(0) }; 2049] };
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
            SIZES.with(|sizes| {
                let count = &sizes[bytes.min(2048)];
                count.set(count.get() + 1);
            });
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

// Repository-maintenance diagnostic: unchanged port and test configuration.
// Counts this thread's allocation/reallocation requests during VM execution,
// including setup/warmup/checksum, never retained heap or clean-host timing.
use kujo::{compiler::Compiler, lexer, parser::Parser, vm::VM};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() == 4, "MODE CASE count|sample");
    let jit = args[1] == "jit";
    assert!(jit || args[1] == "vm");
    let count = args[3] == "count";
    assert!(count || args[3] == "sample");
    let port = std::fs::read_to_string("benchmarks/langarena/port.kujo").unwrap();
    let injected = format!(
        "let arguments := [\"benchmarks/langarena/test.json\", \"{}\", \"--profile\"]",
        args[2]
    );
    assert_eq!(port.matches("let arguments := args()").count(), 1);
    let source = port.replacen("let arguments := args()", &injected, 1);
    let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(jit);
    // Match CLI constant globals; VM::new alone does not install `null`.
    let globals = std::sync::Arc::new(std::sync::Mutex::new(kujo::interpreter::Environment::new()));
    {
        let mut env = globals.lock().unwrap();
        for (name, value) in kujo::builtins::get_builtins() {
            env.set(name, value);
        }
        env.set("null".into(), kujo::interpreter::Value::Null);
    }
    vm.set_globals(globals);
    TRACK.with(|v| v.set(count));
    let result = vm.execute(chunk);
    TRACK.with(|v| v.set(false));
    result.unwrap();
    eprintln!(
        "ALLOCATION_AUDIT mode={} case={} enabled={} requests={} requested_bytes={}",
        args[1],
        args[2],
        count,
        ALLOCATIONS.with(Cell::get),
        REQUESTED_BYTES.with(Cell::get)
    );
}
