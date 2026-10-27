// Exploratory storage model using real Kujo Values, not a runtime implementation.
// Counts requested allocation traffic, including publication where specified.
use kujo::interpreter::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
static TRACK: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
struct Counter;
fn record(n: usize) {
    if TRACK.load(Relaxed) {
        REQUESTS.fetch_add(1, Relaxed);
        BYTES.fetch_add(n as u64, Relaxed);
    }
}
// SAFETY: System receives unchanged allocation/deallocation arguments.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn begin() {
    REQUESTS.store(0, Relaxed);
    BYTES.store(0, Relaxed);
    TRACK.store(true, Relaxed);
}
fn end() -> (u64, u64) {
    TRACK.store(false, Relaxed);
    (REQUESTS.load(Relaxed), BYTES.load(Relaxed))
}
fn integer(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        _ => panic!("unexpected value"),
    }
}
#[derive(Clone)]
struct Chunked {
    chunks: Arc<Vec<Arc<Vec<Value>>>>,
    width: usize,
}
impl Chunked {
    fn new(values: &[Value], width: usize) -> Self {
        Self {
            chunks: Arc::new(values.chunks(width).map(|c| Arc::new(c.to_vec())).collect()),
            width,
        }
    }
    fn get(&self, i: usize) -> &Value {
        &self.chunks[i / self.width][i % self.width]
    }
    fn set(&mut self, i: usize, value: Value) {
        Arc::make_mut(&mut Arc::make_mut(&mut self.chunks)[i / self.width])[i % self.width] = value;
    }
    fn materialize(&self, len: usize) -> Arc<Vec<Value>> {
        let mut values = Vec::with_capacity(len);
        for chunk in self.chunks.iter() {
            values.extend(chunk.iter().cloned());
        }
        assert_eq!(values.len(), len);
        Arc::new(values)
    }
}
fn run(len: usize, writes: usize, width: usize, mode: &str) {
    let initial: Arc<Vec<Value>> = Arc::new((0..len).map(|i| Value::Int(i as i64)).collect());
    let mut model: Vec<i64> = (0..len).map(|i| i as i64).collect();
    let mut flat = initial.clone();
    begin();
    let mut chunked = if mode == "flat" { None } else { Some(Chunked::new(&initial, width)) };
    let setup = end();
    let original_chunks = chunked.clone();
    let mut total = (0, 0);
    let calls = 32;
    for call in 0..calls {
        // The owner and escaped snapshot remain readable until publication.
        let old_flat = flat.clone();
        let old_chunks = chunked.clone();
        let before = model.clone();
        begin();
        if mode == "flat" {
            let mut active = flat.clone();
            for j in 0..writes {
                let i = (call * 17 + j * 31) % len;
                Arc::make_mut(&mut active)[i] = Value::Int((call * writes + j + len) as i64);
            }
            flat = active;
        } else {
            let mut active = chunked.as_ref().unwrap().clone();
            for j in 0..writes {
                let i = (call * 17 + j * 31) % len;
                active.set(i, Value::Int((call * writes + j + len) as i64));
            }
            // Public Value::Array currently requires a contiguous Arc<Vec<Value>>.
            // Omitting this cost models a different host representation contract.
            if mode == "chunked_public_flat" {
                flat = active.materialize(len);
            }
            chunked = Some(active);
        }
        let counts = end();
        total.0 += counts.0;
        total.1 += counts.1;
        for j in 0..writes {
            model[(call * 17 + j * 31) % len] = (call * writes + j + len) as i64;
        }
        for i in 0..len {
            assert_eq!(integer(&initial[i]), i as i64);
            if mode == "flat" || mode == "chunked_public_flat" {
                assert_eq!(integer(&old_flat[i]), before[i]);
                assert_eq!(integer(&flat[i]), model[i]);
            }
            if let Some(old) = &old_chunks {
                assert_eq!(integer(old.get(i)), before[i]);
                assert_eq!(integer(chunked.as_ref().unwrap().get(i)), model[i]);
                assert_eq!(integer(original_chunks.as_ref().unwrap().get(i)), i as i64);
            }
        }
    }
    println!("{{\"mode\":\"{}\",\"length\":{},\"writes\":{},\"chunk_width\":{},\"calls\":{},\"setup_requests\":{},\"setup_bytes\":{},\"requests\":{},\"bytes\":{},\"value_size\":{},\"snapshots_passed\":true}}",mode,len,writes,width,calls,setup.0,setup.1,total.0,total.1,std::mem::size_of::<Value>());
}
fn main() {
    for len in [128, 2048, 32768] {
        for writes in [1, 8, 128] {
            run(len, writes, 0, "flat");
            for width in [32, 128, 512] {
                run(len, writes, width, "chunked");
                run(len, writes, width, "chunked_public_flat");
            }
        }
    }
}
