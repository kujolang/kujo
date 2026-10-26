//! Isolate captured-environment cloning from parsing, module loading and calls.
//! Counts requested traffic, not live bytes. The filtered map is a diagnostic
//! counterfactual only: it must never be used as an execution environment.
use kujo::{interpreter::Value, module::ModuleLoader};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static REQUESTS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}
struct Counter;
fn count(size: usize) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            REQUESTS.with(|v| v.set(v.get() + 1));
            BYTES.with(|v| v.set(v.get() + size));
        }
    });
}
// SAFETY: operations and layouts are forwarded unchanged to System. Accounting
// uses allocation-free thread-local cells and never examines allocated memory.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn measure(label: &str, rounds: usize, mut operation: impl FnMut()) -> (usize, usize) {
    operation(); // Warm TLS and runtime paths outside accounting.
    REQUESTS.with(|v| v.set(0));
    BYTES.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    for _ in 0..rounds {
        operation();
    }
    ACTIVE.with(|v| v.set(false));
    let result = (REQUESTS.with(Cell::get), BYTES.with(Cell::get));
    println!("{label} rounds={rounds} requests={} bytes={}", result.0, result.1);
    result
}
fn main() {
    let directory = std::env::args().nth(1).expect("module directory");
    let mut loader = ModuleLoader::new();
    loader.add_search_path(std::path::Path::new(&directory));
    let module = loader.load_module("import_probe").expect("load fixture");
    let Value::Function(_, _, Some(captured)) = &module.exports["first"] else {
        panic!("expected imported AST function");
    };
    let env = captured.lock().unwrap();
    let globals = &env.scopes[0];
    let native_count = globals.values().filter(|v| matches!(v, Value::NativeFunction(_))).count();
    println!(
        "bindings={} native_bindings={} scopes={}",
        globals.len(),
        native_count,
        env.scopes.len()
    );
    // All samples use the actual module capture, not an invented preload inventory.
    let full = measure("environment", 64, || {
        black_box(env.clone());
    });
    let map = measure("global_map", 64, || {
        black_box(globals.clone());
    });
    let names = measure("binding_names", 64, || {
        for key in globals.keys() {
            black_box(key.clone());
        }
    });
    let natives = measure("native_value_names", 64, || {
        for value in globals.values() {
            if let Value::NativeFunction(name) = value {
                black_box(name.clone());
            }
        }
    });
    let other_values = measure("other_values", 64, || {
        for value in globals.values() {
            if !matches!(value, Value::NativeFunction(_)) {
                black_box(value.clone());
            }
        }
    });
    assert!(full.0 >= map.0 && full.1 >= map.1);
    assert!(map.0 >= names.0 + natives.0 + other_values.0);
    assert!(map.1 >= names.1 + natives.1 + other_values.1);
    assert_eq!(natives.0, native_count * 64);
    // Counterfactual: retaining user bindings alone is NOT semantically valid.
    let non_native: std::collections::HashMap<_, _> = globals
        .iter()
        .filter(|(_, value)| !matches!(value, Value::NativeFunction(_)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    measure("non_native_map_counterfactual", 64, || {
        black_box(non_native.clone());
    });
    // Verify real snapshot isolation independently of the accounting.
    let mut snapshot = env.clone();
    snapshot.define("probe_only".into(), Value::Int(42));
    assert!(env.get("probe_only").is_none());
    assert!(matches!(snapshot.get("probe_only"), Some(Value::Int(42))));
    println!("IMPORT ENVIRONMENT ISOLATION PASS");
}
