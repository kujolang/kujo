use kujo::interpreter::Value;
#[cfg(cached)]
use kujo::jit::jit_indexed_read_cached;
use kujo::jit::{
    jit_enter_function, jit_indexed_read, jit_leave_function, jit_load_literal,
    jit_release_operand, VMContext,
};
use std::{collections::HashMap, hint::black_box, sync::Arc, time::Instant};
fn main() {
    for mode in ["integer_array", "float_array", "float_dictionary", "owned_array"] {
        let (object, index, want, kind) = match mode {
            "integer_array" => (Value::array(vec![Value::Int(7); 64]), Value::Int(17), 7, 0),
            "float_array" => (
                Value::array(vec![Value::Float(1.5); 64]),
                Value::Int(17),
                1.5f64.to_bits() as i64,
                3,
            ),
            "float_dictionary" => (
                Value::Dict(Arc::new(
                    [(Arc::<str>::from("17"), Value::Float(1.5))].into_iter().collect(),
                )),
                Value::Str(Arc::new("17".into())),
                1.5f64.to_bits() as i64,
                3,
            ),
            _ => (Value::array(vec![Value::array(vec![Value::Int(9)]); 64]), Value::Int(17), 0, 4),
        };
        let mut stack = Vec::new();
        let mut globals = HashMap::new();
        let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
        unsafe {
            assert_eq!(jit_enter_function(&mut ctx), 0);
            let object = jit_load_literal(&mut ctx, &object as *const Value as i64);
            let object_kind = ctx.return_value_kind;
            let index = jit_load_literal(&mut ctx, &index as *const Value as i64);
            let index_kind = ctx.return_value_kind;
            let reads = 100000u64;
            #[cfg(cached)]
            let mut cache = [0u64; 2];
            let started = Instant::now();
            let mut checksum = 0u64;
            for _ in 0..reads {
                #[cfg(not(cached))]
                let value =
                    jit_indexed_read(black_box(&mut ctx), object, object_kind, index, index_kind);
                #[cfg(cached)]
                let value = {
                    #[cfg(cold)]
                    {
                        cache = [0u64; 2];
                    }
                    jit_indexed_read_cached(
                        black_box(&mut ctx),
                        object,
                        object_kind,
                        index,
                        index_kind,
                        cache.as_mut_ptr() as i64,
                    )
                };
                assert_eq!(ctx.return_value_kind, kind);
                if kind == 4 {
                    assert_eq!(jit_release_operand(&mut ctx, value, kind), 0);
                } else {
                    checksum = checksum.wrapping_add(black_box(value as u64));
                }
            }
            let elapsed = started.elapsed().as_nanos();
            assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
            if kind != 4 {
                assert_eq!(checksum, (want as u64).wrapping_mul(reads));
            }
            ctx.has_return_value = false;
            jit_leave_function(&mut ctx);
            assert!(ctx.runtime_error.is_none());
            println!("{mode} reads={reads} elapsed_ns={elapsed}");
        }
    }
}
// Build against one identical Kujo library in three variants: no cfg (generic),
// --cfg cached (warm hint), and --cfg cached --cfg cold (reset every read).
// Run the executables with paired_read_cache.kujo in that order plus a JSON path.
// This isolates helper costs; it is not an end-to-end benchmark score.
