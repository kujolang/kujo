// Diagnostic only: process-wide requested allocation traffic, not retained heap.
// Setup/parse are excluded; task workers and callback execution are included.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use kujo::{compiler::Compiler, lexer, parser::Parser, vm::VM};
static TRACK: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static TRACK_ROWS: AtomicBool = AtomicBool::new(false);
static SHORT_ROWS: AtomicU64 = AtomicU64::new(0);
static WIDE_ROWS: AtomicU64 = AtomicU64::new(0);
struct Counter;
fn record(size: usize) {
    if TRACK.load(Relaxed) {
        REQUESTS.fetch_add(1, Relaxed);
        BYTES.fetch_add(size as u64, Relaxed);
        if TRACK_ROWS.load(Relaxed) {
            let value_size = std::mem::size_of::<kujo::interpreter::Value>();
            if size == 128 * value_size { SHORT_ROWS.fetch_add(1, Relaxed); }
            if size == 2048 * value_size { WIDE_ROWS.fetch_add(1, Relaxed); }
        }
    }
}
// SAFETY: all operations delegate to System with the original arguments;
// accounting uses allocation-free atomics, including on worker threads.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 { record(l.size()); unsafe { System.alloc(l) } }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 { record(l.size()); unsafe { System.alloc_zeroed(l) } }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, s: usize) -> *mut u8 { record(s); unsafe { System.realloc(p,l,s) } }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { unsafe { System.dealloc(p,l) } }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() == 3 || (args.len() == 4 && args[3] == "row-buffers"), "MODE SOURCE [row-buffers]");
    assert!(args[1] == "vm" || args[1] == "jit");
    let path = std::fs::canonicalize(&args[2]).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let ast = Parser::new(lexer::tokenize(&source).unwrap()).parse();
    let chunk = Compiler::new().compile(&ast).unwrap();
    let mut vm = VM::new();
    vm.set_jit_enabled(args[1] == "jit");
    vm.add_module_search_path(path.parent().unwrap());
    let globals = Arc::new(Mutex::new(kujo::interpreter::Environment::new()));
    {
        let mut env = globals.lock().unwrap();
        for (name, value) in kujo::builtins::get_builtins() { env.set(name, value); }
        env.set("null".into(), kujo::interpreter::Value::Null);
    }
    vm.set_globals(globals);
    TRACK_ROWS.store(args.len() == 4, Relaxed);
    TRACK.store(true, Relaxed);
    let result = vm.execute_until_suspend(chunk).and_then(|_| vm.run_scheduler_until_complete_with_timeout(Duration::from_secs(60)));
    TRACK.store(false, Relaxed);
    result.unwrap();
    eprintln!("ALLOCATION_AUDIT requests={} requested_bytes={}", REQUESTS.load(Relaxed), BYTES.load(Relaxed));
    if TRACK_ROWS.load(Relaxed) {
        // Size matches are not stack traces. Compare added-call slopes against
        // direct-array and snapshot controls before attributing them to copying.
        eprintln!("ROW_BUFFER_AUDIT value_size={} short={} wide={}",
            std::mem::size_of::<kujo::interpreter::Value>(),
            SHORT_ROWS.load(Relaxed), WIDE_ROWS.load(Relaxed));
    }
}
