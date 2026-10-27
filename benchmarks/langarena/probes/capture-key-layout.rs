// Component experiment only; ManuallyDrop<Null> preserves exact Value layout without pulling runtime destruction into this storage-only probe. No owned payload is leaked.
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use kujo::interpreter::{Value, BindingKind};
use std::mem::ManuallyDrop;
static TRACK: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
struct Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACK.load(Relaxed) { REQUESTS.fetch_add(1, Relaxed); BYTES.fetch_add(layout.size() as u64, Relaxed); }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) { unsafe { System.dealloc(p, layout) } }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        if TRACK.load(Relaxed) { REQUESTS.fetch_add(1, Relaxed); BYTES.fetch_add(size as u64, Relaxed); }
        unsafe { System.realloc(p, old, size) }
    }
}
#[global_allocator] static ALLOCATOR: Counter = Counter;
fn measure<T>(make: impl FnOnce() -> T) -> (T, u64, u64) {
    REQUESTS.store(0, Relaxed); BYTES.store(0, Relaxed); TRACK.store(true, Relaxed);
    let result = make();
    TRACK.store(false, Relaxed);
    (result, REQUESTS.load(Relaxed), BYTES.load(Relaxed))
}
fn main() {
    println!("value_size={} value_align={}", std::mem::size_of::<Value>(), std::mem::align_of::<Value>());
    for width in [0, 128, 2048, 4096, 8192, 12288, 16384] {
        let mut owner: HashMap<String, ()> = (0..width).map(|n| (format!("extra_{n}"), ())).collect();
        let (private, map_requests, map_bytes) = measure(|| {
            let mut map = HashMap::new(); map.reserve(owner.len());
            for name in owner.keys() { map.insert(name.clone(), ManuallyDrop::new(Value::Null)); }
            map
        });
        let (keys, set_requests, set_bytes) = measure(|| {
            let mut set = HashSet::new(); set.reserve(owner.len());
            for name in owner.keys() { set.insert(name.clone()); }
            set
        });
        let (kinds, kind_requests, kind_bytes) = measure(|| {
            let mut map = HashMap::new(); map.reserve(owner.len());
            for name in owner.keys() { map.insert(name.clone(), BindingKind::Mutable); }
            map
        });
        assert!(owner.keys().all(|key| kinds.contains_key(key)));
        assert_eq!(map_requests, kind_requests);
        assert!(kind_bytes <= map_bytes);
        assert_eq!(private.len(), keys.len());
        assert!(private.keys().all(|key| keys.contains(key)));
        assert!(private.values().all(|value| matches!(&**value, Value::Null)));
        // The key snapshot must not acquire a later owner definition.
        owner.insert("later".into(), ());
        assert!(!keys.contains("later"));
        if width > 0 {
            owner.remove("extra_0");
            assert!(keys.contains("extra_0"));
            assert!(private.contains_key("extra_0"));
        }
        assert_eq!(map_requests, set_requests, "key ownership is deliberately unchanged");
        assert!(set_bytes <= map_bytes);
        println!("width={width} map_requests={map_requests} map_bytes={map_bytes} set_requests={set_requests} set_bytes={set_bytes} kind_requests={kind_requests} kind_bytes={kind_bytes}");
    }
}
