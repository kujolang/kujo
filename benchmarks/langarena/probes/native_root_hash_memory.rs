// Allocation/retention control for the root table's exact u64->usize map shape.
// No runtime change; counter snapshots exclude printing and validate final drop.
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
static LIVE: AtomicUsize = AtomicUsize::new(0);
static REQUESTS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
struct Count;
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
            REQUESTS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
            REQUESTS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(p, old, size) };
        if !new.is_null() {
            LIVE.fetch_add(size, Relaxed);
            LIVE.fetch_sub(old.size(), Relaxed);
            REQUESTS.fetch_add(1, Relaxed);
            BYTES.fetch_add(size, Relaxed);
        }
        new
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Count = Count;
#[derive(Default)]
struct Multiplied(nohash_hasher::NoHashHasher<u64>);
impl Hasher for Multiplied {
    fn finish(&self) -> u64 {
        self.0.finish().wrapping_mul(0x9e3779b97f4a7c15)
    }
    fn write(&mut self, b: &[u8]) {
        self.0.write(b);
    }
    fn write_u64(&mut self, v: u64) {
        self.0.write_u64(v);
    }
}
fn snapshot() -> [usize; 3] {
    [REQUESTS.load(Relaxed), BYTES.load(Relaxed), LIVE.load(Relaxed)]
}
fn run<S: BuildHasher + Default>(round: usize, name: &str, live: u64) {
    let before = snapshot();
    let mut map = HashMap::<u64, usize, S>::default();
    for id in 1..=live {
        assert!(map.insert(id, id as usize).is_none());
    }
    let setup = snapshot();
    let mut next = live + 1;
    for _ in 0..4096 {
        map.insert(next, 9);
        assert_eq!(map.remove(&next), Some(9));
        next += 1;
    }
    let warm = snapshot();
    for _ in 0..100000 {
        assert!(map.insert(next, 9).is_none());
        assert_eq!(map.get(&next), Some(&9));
        assert_eq!(map.remove(&next), Some(9));
        next += 1;
    }
    let measured = snapshot();
    assert_eq!(map.len(), live as usize);
    for id in 1..=live {
        assert_eq!(map.get(&id), Some(&(id as usize)));
    }
    drop(map);
    let after = snapshot();
    assert_eq!(after[2], before[2], "table must release retained storage");
    println!("{{\"round\":{round},\"hasher\":\"{name}\",\"live\":{live},\"before\":{before:?},\"setup\":{setup:?},\"warm\":{warm:?},\"measured\":{measured:?},\"after\":{after:?}}}");
}
fn main() {
    for round in 0..3 {
        for live in [1, 64, 2048] {
            if round % 2 == 0 {
                run::<nohash_hasher::BuildNoHashHasher<u64>>(round, "identity", live);
                run::<BuildHasherDefault<Multiplied>>(round, "multiply", live);
            } else {
                run::<BuildHasherDefault<Multiplied>>(round, "multiply", live);
                run::<nohash_hasher::BuildNoHashHasher<u64>>(round, "identity", live);
            }
        }
    }
}
