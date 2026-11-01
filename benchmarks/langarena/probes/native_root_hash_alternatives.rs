// Standalone root-map mechanism screen. No runtime implementation changes.
// Retained keys, removed keys, rotating reads and high handle ranges are checked.
use std::{
    collections::HashMap,
    hash::{BuildHasher, BuildHasherDefault, Hasher},
    hint::black_box,
    time::Instant,
};

#[derive(Default)]
struct Mixed<const KIND: u8>(u64);
impl<const KIND: u8> Hasher for Mixed<KIND> {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        panic!("u64 keys required");
    }
    fn write_u64(&mut self, value: u64) {
        self.0 = match KIND {
            0 => value.wrapping_mul(0x9e3779b97f4a7c15),
            1 => value ^ (value << 7),
            2 => value ^ (value << 13),
            3 => value ^ value.rotate_left(7),
            _ => unreachable!(),
        };
    }
}

fn run<S: BuildHasher + Default>(round: usize, name: &str, base: u64, live: u64, mode: u8) {
    let mut map = HashMap::<u64, usize, S>::default();
    for id in 1..=live {
        assert!(map.insert(base + id, id as usize).is_none());
    }
    let setup_capacity = map.capacity();
    let mut next = base + live + 1;
    for _ in 0..4096 {
        assert!(map.insert(next, 9).is_none());
        assert_eq!(map.remove(&next), Some(9));
        next += 1;
    }
    let warm_capacity = map.capacity();
    let operations = 200_000usize;
    let start = Instant::now();
    let mut sum = 0usize;
    for operation in 0..operations {
        match mode {
            0 => sum += *map.get(&black_box(base + 1)).unwrap(),
            1 => {
                let id = operation as u64 % live + 1;
                sum += *map.get(&black_box(base + id)).unwrap();
            }
            2 => {
                let key = black_box(next);
                assert!(map.insert(key, 9).is_none());
                sum += *map.get(&key).unwrap();
                assert_eq!(map.remove(&key), Some(9));
                next += 1;
            }
            _ => unreachable!(),
        }
    }
    let elapsed_ns = start.elapsed().as_nanos();
    let expected = match mode {
        0 => operations,
        1 => (0..operations).map(|i| i % live as usize + 1).sum(),
        2 => operations * 9,
        _ => unreachable!(),
    };
    assert_eq!(sum, expected);
    assert_eq!(map.len(), live as usize);
    for id in 1..=live {
        assert_eq!(map.get(&(base + id)), Some(&(id as usize)));
    }
    // Removed IDs must stay absent even when buckets were reused by churn.
    for key in [base, base + live + 1, next - 1, next] {
        assert_eq!(map.get(&key), None);
    }
    let final_capacity = map.capacity();
    println!(
        "{{\"round\":{round},\"hasher\":\"{name}\",\"base\":{base},\"live\":{live},\"mode\":{mode},\"operations\":{operations},\"checksum\":{sum},\"elapsed_ns\":{elapsed_ns},\"setup_capacity\":{setup_capacity},\"warm_capacity\":{warm_capacity},\"final_capacity\":{final_capacity}}}"
    );
}

fn main() {
    for round in 0..7 {
        let bases = if round % 2 == 0 { [0, 1 << 32, 1 << 63] } else { [1 << 63, 1 << 32, 0] };
        for base in bases {
            for live in if round % 2 == 0 { [1, 64, 2048] } else { [2048, 64, 1] } {
                for mode in 0..3 {
                    let mut names = ["identity", "multiply", "xor7", "xor13", "xor_rotate7"];
                    names.rotate_left(round % 5);
                    if round % 2 == 1 {
                        names.reverse();
                    }
                    for name in names {
                        match name {
                            "identity" => run::<nohash_hasher::BuildNoHashHasher<u64>>(
                                round, name, base, live, mode,
                            ),
                            "multiply" => {
                                run::<BuildHasherDefault<Mixed<0>>>(round, name, base, live, mode)
                            }
                            "xor7" => {
                                run::<BuildHasherDefault<Mixed<1>>>(round, name, base, live, mode)
                            }
                            "xor13" => {
                                run::<BuildHasherDefault<Mixed<2>>>(round, name, base, live, mode)
                            }
                            _ => run::<BuildHasherDefault<Mixed<3>>>(round, name, base, live, mode),
                        }
                    }
                }
            }
        }
    }
}
