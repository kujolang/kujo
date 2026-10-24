use kujo::interpreter::Value;
use std::{hint::black_box, time::Instant};
#[repr(C)]
#[derive(Clone, Copy)]
struct Scalar {
    bits: i64,
    kind: i64,
}
fn encode(v: &Value) -> Option<Scalar> {
    match v {
        Value::Int(v) => Some(Scalar { bits: *v, kind: 0 }),
        Value::Bool(v) => Some(Scalar { bits: i64::from(*v), kind: 1 }),
        Value::Null => Some(Scalar { bits: 0, kind: 2 }),
        Value::Float(v) => Some(Scalar { bits: v.to_bits() as i64, kind: 3 }),
        _ => None,
    }
}
fn main() {
    for n in [128usize, 900] {
        let values: Vec<Value> = (0..n * n).map(|i| Value::Float(((i * 17) % 97) as f64)).collect();
        let started = Instant::now();
        let packed: Vec<Scalar> = values.iter().map(|v| encode(v).unwrap()).collect();
        println!(
            "PACK n={n} elapsed_ns={} generic_bytes={} packed_bytes={}",
            started.elapsed().as_nanos(),
            values.len() * std::mem::size_of::<Value>(),
            packed.len() * std::mem::size_of::<Scalar>()
        );
        for column_major in [false, true] {
            let mut expected = None;
            for round in 0..7 {
                for slot in 0..2 {
                    let variant = (round + slot) % 2;
                    let started = Instant::now();
                    let mut checksum = 0u64;
                    for _ in 0..8 {
                        for a in 0..n {
                            for b in 0..n {
                                let i = if column_major { b * n + a } else { a * n + b };
                                let v = if variant == 0 {
                                    encode(&black_box(&values)[i]).unwrap()
                                } else {
                                    black_box(&packed)[i]
                                };
                                checksum =
                                    checksum.wrapping_add(black_box(v.bits as u64 ^ v.kind as u64));
                            }
                        }
                    }
                    let elapsed = started.elapsed().as_nanos();
                    if let Some(want) = expected {
                        assert_eq!(checksum, want);
                    } else {
                        expected = Some(checksum);
                    }
                    println!("READ n={n} column_major={column_major} round={round} variant={variant} elapsed_ns={elapsed} checksum={checksum}");
                }
            }
        }
    }
}
