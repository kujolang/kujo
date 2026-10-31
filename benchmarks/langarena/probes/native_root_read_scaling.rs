// Diagnostic of indexed reads versus live-root count, not a language speed score.
// Link against the frozen accepted library. Setup, warmup and root teardown are
// outside the timed interval. Owned rows include validation and release per read.
use kujo::interpreter::Value;
use kujo::jit::{
    jit_enter_function, jit_indexed_read, jit_indexed_read_cached, jit_leave_function,
    jit_load_literal, jit_release_operand, VMContext,
};
use std::{collections::HashMap, hint::black_box, sync::Arc, time::Instant};

fn measure(round: usize, live: usize, rotating: bool, cached: bool, row: bool) {
    let element = if row { Value::array(vec![Value::Int(9)]) } else { Value::Int(9) };
    let values = Arc::new(vec![element; 64]);
    let object = Value::Array(Arc::clone(&values));
    let mut stack = Vec::new();
    let mut globals = HashMap::new();
    let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
    // SAFETY: context/backing storage and literals stay live and exclusively
    // borrowed. Cache is an aligned, initialized 16-byte OwnedReadCache ABI hint;
    // handles are produced by the runtime, never fabricated or dereferenced.
    unsafe {
        assert_eq!(jit_enter_function(&mut ctx), 0);
        let roots: Vec<_> = (0..live)
            .map(|_| {
                let root = jit_load_literal(&mut ctx, &object as *const Value as i64);
                assert_eq!(ctx.return_value_kind, 4);
                assert!(ctx.runtime_error.is_none());
                root
            })
            .collect();
        let mut cache = [0u64; 2];
        let mut read = |ctx: &mut VMContext, i: usize| -> u64 {
            let root = roots[if rotating { i % live } else { 0 }];
            let result = if cached {
                jit_indexed_read_cached(
                    black_box(&mut *ctx),
                    black_box(root),
                    4,
                    17,
                    0,
                    cache.as_mut_ptr() as i64,
                )
            } else {
                jit_indexed_read(black_box(&mut *ctx), black_box(root), 4, 17, 0)
            };
            assert_eq!(ctx.return_value_kind, if row { 4 } else { 0 });
            assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
            if row {
                let value = jit_indexed_read(ctx, result, 4, 0, 0);
                assert_eq!(ctx.return_value_kind, 0);
                assert_eq!(value, 9);
                assert_eq!(jit_release_operand(ctx, result, 4), 0);
                assert!(ctx.runtime_error.is_none());
                value as u64
            } else {
                assert_eq!(result, 9);
                result as u64
            }
        };
        for i in 0..4096 {
            assert_eq!(read(&mut ctx, i), 9);
        }
        let reads = 100_000usize;
        let start = Instant::now();
        let mut sum = 0u64;
        for i in 0..reads {
            sum = sum.wrapping_add(black_box(read(&mut ctx, i)));
        }
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(sum, 9 * reads as u64);
        // A stale cache hint must not keep roots alive after the frame ends.
        ctx.has_return_value = false;
        jit_leave_function(&mut ctx);
        assert!(ctx.runtime_error.is_none());
        assert_eq!(Arc::strong_count(&values), 2, "frame must release all root values");
        println!("{{\"round\":{round},\"live_roots\":{live},\"rotating\":{rotating},\"cached\":{cached},\"row\":{row},\"reads\":{reads},\"checksum\":{sum},\"elapsed_ns\":{elapsed}}}");
    }
}
fn main() {
    for round in 0..3 {
        let widths = if round % 2 == 0 { [1, 64, 2048] } else { [2048, 64, 1] };
        for live in widths {
            for row in [false, true] {
                for rotating in [false, true] {
                    for cached in if round % 2 == 0 { [false, true] } else { [true, false] } {
                        measure(round, live, rotating, cached, row);
                    }
                }
            }
        }
    }
}
