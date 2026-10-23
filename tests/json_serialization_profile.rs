//! Opt-in allocation accounting for JSON serialization; no wall-clock assertions.
use kujo::interpreter::{DictMap, Value};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

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

fn object(entries: Vec<(&str, Value)>) -> Value {
    let values: DictMap = entries.into_iter().map(|(key, value)| (key.into(), value)).collect();
    Value::Dict(Arc::new(values))
}

fn measure(rows: usize) -> (usize, usize, usize) {
    let coordinates = Value::Array(Arc::new(
        (0..rows)
            .map(|index| {
                object(vec![
                    ("x", Value::Float(0.12345678)),
                    ("y", Value::Float(0.23456789)),
                    ("z", Value::Float(0.34567891)),
                    ("name", Value::Str(Arc::new(format!("row {index}")))),
                    (
                        "opts",
                        object(vec![(
                            "1",
                            Value::Array(Arc::new(vec![Value::Int(1), Value::Bool(true)])),
                        )]),
                    ),
                ])
            })
            .collect(),
    ));
    let input = object(vec![
        ("coordinates", coordinates),
        ("info", Value::Str(Arc::new("some info".into()))),
    ]);

    // Initialize TLS before enabling accounting; exclude fixture creation,
    // assertions, parsing, printing and destruction of the result buffer.
    ALLOCATIONS.with(|count| count.set(0));
    REQUESTED_BYTES.with(|count| count.set(0));
    TRACK.with(|enabled| enabled.set(true));
    let result = kujo::builtins::to_json(&input);
    TRACK.with(|enabled| enabled.set(false));
    let allocations = ALLOCATIONS.with(Cell::get);
    let bytes = REQUESTED_BYTES.with(Cell::get);
    let text = result.unwrap();
    let decoded: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(decoded["coordinates"].as_array().unwrap().len(), rows);
    assert_eq!(decoded["coordinates"][rows - 1]["name"], format!("row {}", rows - 1));
    (allocations, bytes, text.len())
}

#[test]
fn serialization_allocation_budget_excludes_an_owned_dom_copy() {
    let rows = 1000;
    let (allocations, _, _) = measure(rows);
    // The historical owned-DOM path used 12,025 operations on this fixture.
    // Leave implementation/platform headroom while detecting its reintroduction.
    assert!(allocations < rows * 8 + 64, "serialization allocated {allocations} times");
}

#[test]
#[ignore = "allocation diagnostic; run explicitly with --ignored --nocapture"]
fn profile_json_serialization_allocations() {
    for rows in [1, 100, 1000] {
        for repeat in 0..3 {
            let (allocations, bytes, output) = measure(rows);
            println!("rows={rows} repeat={repeat} allocations={allocations} requested_bytes={bytes} output_bytes={output}");
        }
    }
}
