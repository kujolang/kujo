//! Allocation accounting for bytecode call setup; no wall-clock assertions.
use kujo::{compiler::Compiler, interpreter::Value, lexer, parser::Parser, vm::VM};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

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

fn measure(calls: usize) -> (usize, usize) {
    let source = format!(
        "func step(x) {{ return x + 1 }} mut x := 0 {} return x",
        "x = step(x)\n".repeat(calls)
    );
    let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(false);
    ALLOCATIONS.with(|count| count.set(0));
    REQUESTED_BYTES.with(|count| count.set(0));
    TRACK.with(|enabled| enabled.set(true));
    let result = vm.execute(chunk);
    TRACK.with(|enabled| enabled.set(false));
    let allocations = ALLOCATIONS.with(Cell::get);
    let bytes = REQUESTED_BYTES.with(Cell::get);
    assert!(matches!(result.unwrap(), Value::Int(value) if value == calls as i64));
    (allocations, bytes)
}

#[test]
fn call_allocation_budget_excludes_per_call_placeholder_chunks() {
    // Warm up process/TLS state before comparing the allocation slope. The
    // pre-fix path used 14 allocations and 1,369 requested bytes per extra call
    // on this fixture, including an otherwise unused empty code allocation.
    measure(1);
    let (small_count, small_bytes) = measure(100);
    let (large_count, large_bytes) = measure(1000);
    assert!(large_count - small_count <= 13 * 900);
    // Byte size is diagnostic only: Rust layouts may differ by target/toolchain.
    assert!(large_bytes > small_bytes);
}

#[test]
#[ignore = "allocation diagnostic; run explicitly with --ignored --nocapture"]
fn profile_bytecode_call_allocations() {
    for calls in [1, 100, 1000] {
        for repeat in 0..3 {
            let (allocations, bytes) = measure(calls);
            println!(
                "calls={calls} repeat={repeat} allocations={allocations} requested_bytes={bytes}"
            );
        }
    }
}

fn measure_fixed_writes(width: usize, writes: usize, string_key: bool) -> (usize, usize) {
    let entries = (0..width).map(|i| format!("\"{i}\":0")).collect::<Vec<_>>().join(",");
    let key = if string_key { "\"0\"" } else { "0" };
    let source = format!(
        "func update() {{ mut row := {{{entries}}} {} return row[0] }} return update()",
        format!("row[{key}] = 7\n").repeat(writes),
    );
    let mut parser = Parser::new(lexer::tokenize(&source).unwrap());
    let chunk = Compiler::new().compile(&parser.parse()).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(false);
    ALLOCATIONS.with(|count| count.set(0));
    REQUESTED_BYTES.with(|count| count.set(0));
    TRACK.with(|enabled| enabled.set(true));
    let result = vm.execute(chunk);
    TRACK.with(|enabled| enabled.set(false));
    let counts = (ALLOCATIONS.with(Cell::get), REQUESTED_BYTES.with(Cell::get));
    assert!(matches!(result.unwrap(), Value::Int(7)));
    counts
}

#[test]
fn fixed_dictionary_write_bytes_do_not_scale_with_row_width() {
    for string_key in [false, true] {
        measure_fixed_writes(8, 1, string_key);
        let small = measure_fixed_writes(8, 10, string_key);
        let small_many = measure_fixed_writes(8, 100, string_key);
        let wide = measure_fixed_writes(256, 10, string_key);
        let wide_many = measure_fixed_writes(256, 100, string_key);
        let narrow_growth = small_many.1 - small.1;
        let wide_growth = wide_many.1 - wide.1;
        assert!(wide_growth <= narrow_growth,
            "string_key={string_key}: 90 writes allocated {narrow_growth} bytes for 8 fields, {wide_growth} for 256 fields");
    }
}

#[test]
#[ignore = "allocation diagnostic; run explicitly with --ignored --nocapture"]
fn profile_fixed_dictionary_writes() {
    for string_key in [false, true] {
        for width in [8, 256] {
            for writes in [1, 100, 1000] {
                for repeat in 0..3 {
                    let (allocations, bytes) = measure_fixed_writes(width, writes, string_key);
                    println!("string_key={string_key} width={width} writes={writes} repeat={repeat} allocations={allocations} requested_bytes={bytes}");
                }
            }
        }
    }
}
