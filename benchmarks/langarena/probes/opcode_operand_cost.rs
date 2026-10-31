// Isolates metadata extraction, not full VM dispatch or language performance.
// Link both paths against the same accepted runtime. black_box prevents the
// compiler from replacing the owned opcode with a borrowed operand extraction.
use kujo::bytecode::OpCode;
use std::{hint::black_box, time::Instant};

#[inline(never)]
fn owned(op: &OpCode) -> usize {
    match black_box(op.clone()) {
        OpCode::LoadLocal(n) | OpCode::LoadConst(n) => n,
        _ => panic!("unexpected opcode"),
    }
}

#[inline(never)]
fn borrowed(op: &OpCode) -> usize {
    match black_box(op) {
        OpCode::LoadLocal(n) | OpCode::LoadConst(n) => *n,
        _ => panic!("unexpected opcode"),
    }
}

fn main() {
    let instructions: Vec<_> = (0..256)
        .map(|n| if n % 2 == 0 { OpCode::LoadLocal(n) } else { OpCode::LoadConst(n) })
        .collect();
    let reads = 2_000_000usize;
    let expected: usize = (0..reads).map(|n| n % 256).sum();
    println!("round,path,reads,checksum,ns_per_read");
    for round in 0..8 {
        for copy in if round % 2 == 0 { [true, false] } else { [false, true] } {
            let read = if copy { owned } else { borrowed };
            for op in &instructions {
                assert_eq!(owned(op), borrowed(op));
            }
            let start = Instant::now();
            let mut checksum = 0;
            for n in 0..reads {
                checksum += read(black_box(&instructions[n % instructions.len()]));
            }
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(checksum, expected);
            println!(
                "{round},{},{reads},{checksum},{}",
                if copy { "owned" } else { "borrowed" },
                elapsed as f64 / reads as f64
            );
        }
    }
}
