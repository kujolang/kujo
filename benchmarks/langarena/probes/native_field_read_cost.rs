// Direct helper diagnostic, not a language benchmark or clean-host speed score.
// Root setup and warmup are excluded; the same source must link both libraries.
use kujo::interpreter::Value;
use kujo::jit::{
    jit_enter_function, jit_field_read, jit_indexed_read, jit_leave_function, jit_load_literal,
    jit_release_operand, VMContext,
};
use std::{collections::HashMap, hint::black_box, sync::Arc, time::Instant};

fn main() {
    for round in 0..6 {
        let mut shapes = vec!["int", "float", "bool", "null", "row"];
        if round % 2 != 0 {
            shapes.reverse();
        }
        for shape in shapes {
            for field in if round % 2 == 0 { [false, true] } else { [true, false] } {
                let (element, expected, expected_kind) = match shape {
                    "int" => (Value::Int(7), 7, 0),
                    "float" => (Value::Float(-0.0), (-0.0f64).to_bits() as i64, 3),
                    "bool" => (Value::Bool(true), 1, 1),
                    "null" => (Value::Null, 0, 2),
                    _ => (Value::array(vec![Value::Int(9)]), 0, 4),
                };
                let object = if field {
                    Value::Struct {
                        name: "Holder".into(),
                        fields: Arc::new([("value".into(), element)].into_iter().collect()),
                    }
                } else {
                    Value::array(vec![element])
                };
                let name = String::from("value");
                let mut stack = Vec::new();
                let mut globals = HashMap::new();
                let mut ctx = VMContext::new(&mut stack, std::ptr::null_mut(), &mut globals);
                // SAFETY: all context storage and name metadata remain live and
                // exclusively borrowed throughout this synchronous scope. Roots
                // come from the runtime and owned results are released once.
                unsafe {
                    assert_eq!(jit_enter_function(&mut ctx), 0);
                    let root = jit_load_literal(&mut ctx, &object as *const Value as i64);
                    let root_kind = ctx.return_value_kind;
                    let read = |ctx: &mut VMContext| {
                        if field {
                            jit_field_read(ctx, root, root_kind, &name as *const String as i64)
                        } else {
                            jit_indexed_read(ctx, root, root_kind, 0, 0)
                        }
                    };
                    for _ in 0..100 {
                        let result = read(&mut ctx);
                        assert_eq!(ctx.return_value_kind, expected_kind);
                        assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
                        if expected_kind == 4 {
                            assert_eq!(jit_indexed_read(&mut ctx, result, 4, 0, 0), 9);
                            assert_eq!(ctx.return_value_kind, 0);
                            assert_eq!(jit_release_operand(&mut ctx, result, 4), 0);
                        } else {
                            assert_eq!(result, expected);
                        }
                    }
                    let reads = 200_000u64;
                    let started = Instant::now();
                    let mut checksum = 0u64;
                    for _ in 0..reads {
                        let result = read(black_box(&mut ctx));
                        assert_eq!(ctx.return_value_kind, expected_kind);
                        if expected_kind == 4 {
                            assert_eq!(jit_release_operand(&mut ctx, result, 4), 0);
                        } else {
                            checksum = checksum.wrapping_add(black_box(result as u64));
                        }
                    }
                    let elapsed_ns = started.elapsed().as_nanos();
                    assert!(ctx.runtime_error.is_none(), "{:?}", ctx.runtime_error);
                    if expected_kind != 4 {
                        assert_eq!(checksum, (expected as u64).wrapping_mul(reads));
                    }
                    assert!(stack.is_empty(), "unexpected method receiver");
                    ctx.has_return_value = false;
                    jit_leave_function(&mut ctx);
                    assert!(ctx.runtime_error.is_none());
                    println!("round={round} shape={shape} field={field} reads={reads} elapsed_ns={elapsed_ns}");
                }
            }
        }
    }
}
