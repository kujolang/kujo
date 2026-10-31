// Diagnostic only: exact allocator requests and retained bytes for empty code owners.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use kujo::bytecode::BytecodeChunk;

struct Counted;
static TRACK: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() && TRACK.load(Ordering::Relaxed) {
            REQUESTS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACK.load(Ordering::Relaxed) {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let result = System.realloc(ptr, old, size);
        if !result.is_null() && TRACK.load(Ordering::Relaxed) {
            REQUESTS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size, Ordering::Relaxed);
            LIVE.fetch_add(size, Ordering::Relaxed);
            LIVE.fetch_sub(old.size(), Ordering::Relaxed);
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;

fn main() {
    for count in [0, 1, 128, 2048] {
        // Capacity is outside accounting; every tracked owner dies before tracking stops.
        let mut owners = Vec::with_capacity(count);
        REQUESTS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        LIVE.store(0, Ordering::Relaxed);
        TRACK.store(true, Ordering::Relaxed);
        for _ in 0..count { owners.push(std::hint::black_box(BytecodeChunk::new())); }
        std::hint::black_box(&owners);
        let retained = LIVE.load(Ordering::Relaxed);
        owners.clear();
        TRACK.store(false, Ordering::Relaxed);
        let remaining = LIVE.load(Ordering::Relaxed);
        assert_eq!(remaining, 0, "code owners must release tracked storage");
        println!("CODE count={count} requests={} bytes={} retained={retained} after_drop={remaining}",
            REQUESTS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed));
    }
}
