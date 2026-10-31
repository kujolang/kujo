// Root-table mechanism control only. Does not change Kujo or certify a hasher.
use std::{
    collections::HashMap,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hasher},
    hint::black_box,
    time::Instant,
};
#[derive(Default)]
struct Multiplied(u64);
impl Hasher for Multiplied {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        panic!("u64 keys required");
    }
    fn write_u64(&mut self, value: u64) {
        self.0 = value.wrapping_mul(0x9e3779b97f4a7c15);
    }
}
fn run<S: BuildHasher + Default>(round: usize, name: &str, live: u64, cycle: bool) {
    let mut map = HashMap::<u64, usize, S>::default();
    for key in 1..=live {
        assert!(map.insert(key, key as usize).is_none());
    }
    let mut next = live + 1;
    for _ in 0..4096 {
        map.insert(next, 9);
        assert_eq!(map.remove(&next), Some(9));
        next += 1;
    }
    let operations = 200_000;
    let start = Instant::now();
    let mut sum = 0usize;
    for _ in 0..operations {
        if cycle {
            let key = black_box(next);
            assert!(map.insert(key, 9).is_none());
            sum += *map.get(&key).unwrap();
            assert_eq!(map.remove(&key), Some(9));
            next += 1;
        } else {
            sum += *map.get(&black_box(1)).unwrap();
        }
    }
    let elapsed = start.elapsed().as_nanos();
    assert_eq!(sum, operations * if cycle { 9 } else { 1 });
    assert_eq!(map.len(), live as usize);
    for key in 1..=live {
        assert_eq!(map.get(&key), Some(&(key as usize)));
    }
    println!("{{\"round\":{round},\"hasher\":\"{name}\",\"live\":{live},\"cycle\":{cycle},\"operations\":{operations},\"checksum\":{sum},\"elapsed_ns\":{elapsed}}}");
}
fn main() {
    for round in 0..7 {
        for live in if round % 2 == 0 { [1, 64, 2048] } else { [2048, 64, 1] } {
            for cycle in [false, true] {
                for name in if round % 2 == 0 {
                    ["identity", "multiply", "standard"]
                } else {
                    ["standard", "multiply", "identity"]
                } {
                    match name {
                        "identity" => {
                            run::<nohash_hasher::BuildNoHashHasher<u64>>(round, name, live, cycle)
                        }
                        "multiply" => {
                            run::<BuildHasherDefault<Multiplied>>(round, name, live, cycle)
                        }
                        _ => run::<BuildHasherDefault<DefaultHasher>>(round, name, live, cycle),
                    }
                }
            }
        }
    }
}
