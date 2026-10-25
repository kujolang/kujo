// Diagnostic-only phase/ownership allocator. See LOCAL_ISOLATION.md.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static REQUESTED_BYTES: Cell<usize> = const { Cell::new(0) };
}

thread_local! {
    static PHASE_REQUESTS: [Cell<usize>; 6] = const { [const { Cell::new(0) }; 6] };
    static PHASE_BYTES: [Cell<usize>; 6] = const { [const { Cell::new(0) }; 6] };
}
unsafe extern "C" {
    fn getenv(name: *const std::ffi::c_char) -> *mut std::ffi::c_char;
}
struct CountingAllocator;

fn record(bytes: usize) {
    let _ = TRACK.try_with(|enabled| {
        if enabled.get() {
            // Single-thread probe only. libc getenv does not allocate; only the
            // diagnostic source changes this private phase marker between calls.
            let p = unsafe { getenv(c"KUJO_ALLOCATION_PHASE".as_ptr()) };
            let phase = if p.is_null() {
                0
            } else {
                (unsafe { *p } as u8).saturating_sub(b'0').min(5) as usize
            };
            PHASE_REQUESTS.with(|a| a[phase].set(a[phase].get() + 1));
            PHASE_BYTES.with(|a| a[phase].set(a[phase].get() + bytes));

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

// Repository-maintenance diagnostic: unchanged port and test configuration.
// Counts this thread's allocation/reallocation requests during VM execution,
// including setup/warmup/checksum, never retained heap or clean-host timing.
use kujo::{compiler::Compiler, lexer, parser::Parser, vm::VM};
include!("local_isolation_micro.rs");
include!("local_isolation_calls.rs");
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "micro") {
        micro_controls();
        call_controls();
        return;
    }
    assert!(args.len() == 4, "MODE SOURCE count|sample");
    let jit = args[1] == "jit";
    assert!(jit || args[1] == "vm");
    let count = args[3] == "count";
    assert!(count || args[3] == "sample");
    let source = std::fs::read_to_string(&args[2]).unwrap();
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
    kujo::benchmarks::profiler::runtime::start().unwrap();
    TRACK.with(|v| v.set(count));
    let result = vm.execute(chunk);
    TRACK.with(|v| v.set(false));
    result.unwrap();
    eprintln!("COUNTERS {}", kujo::benchmarks::profiler::runtime::snapshot("success"));
    for phase in 0..6 {
        eprintln!(
            "PHASE {phase} requests={} bytes={}",
            PHASE_REQUESTS.with(|a| a[phase].get()),
            PHASE_BYTES.with(|a| a[phase].get())
        );
    }

    eprintln!(
        "ALLOCATION_AUDIT mode={} case={} enabled={} requests={} requested_bytes={}",
        args[1],
        args[2],
        count,
        ALLOCATIONS.with(Cell::get),
        REQUESTED_BYTES.with(Cell::get)
    );
}
