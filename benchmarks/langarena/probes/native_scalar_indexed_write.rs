// Direct-helper isolation, never an elapsed LangArena score. Build this exact
// source against each frozen release library; compare ordinary kernels separately.
use kujo::interpreter::Value;
use kujo::jit::{jit_enter_function, jit_indexed_read, jit_indexed_write, jit_leave_function, jit_load_literal, VMContext};
use std::{collections::HashMap, hint::black_box, time::Instant};
fn main() {
    for round in 0..5 {
        for (shape, payload, kind) in [("integer", 27, 0), ("float", (-0.0f64).to_bits() as i64, 3), ("boolean", 1, 1), ("null", 0, 2)] {
            for location in [0, 1] {
                let mut names = HashMap::from([(7u64, "data".to_owned())]);
                let mut slots = vec![Value::array(vec![Value::Int(10); 16])];
                let mut ctx = VMContext::new(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
                ctx.var_names_ptr = &mut names;
                ctx.local_slots_ptr = &mut slots;
                // SAFETY: metadata, physical slots and exclusive context remain
                // live throughout; root is issued by this activation. No borrowed
                // value or backing pointer survives a helper call.
                unsafe {
                    assert_eq!(jit_enter_function(&mut ctx), 0);
                    let (target, target_kind) = if location == 1 {
                        let value = Value::array(vec![Value::Int(10); 16]);
                        let root = jit_load_literal(&mut ctx, &value as *const Value as i64);
                        (root, ctx.return_value_kind)
                    } else { (0, 0) };
                    for _ in 0..1000 {
                        assert_eq!(jit_indexed_write(&mut ctx, location, target, target_kind, 0, 7, 3, 0, payload, kind), 0);
                    }
                    let started = Instant::now();
                    for _ in 0..200_000 {
                        assert_eq!(jit_indexed_write(black_box(&mut ctx), location, target, target_kind, 0, 7, black_box(3), 0, black_box(payload), kind), 0);
                    }
                    let elapsed = started.elapsed().as_nanos();
                    assert!(ctx.runtime_error.is_none());
                    if location == 1 {
                        assert_eq!(jit_indexed_read(&mut ctx, target, target_kind, 3, 0), payload);
                        assert_eq!(ctx.return_value_kind, kind);
                    } else {
                        let Value::Array(values) = &slots[0] else { panic!() };
                        match &values[3] {
                            Value::Int(v) => assert_eq!((*v, kind), (payload, 0)),
                            Value::Bool(v) => assert_eq!((i64::from(*v), kind), (payload, 1)),
                            Value::Null => assert_eq!(kind, 2),
                            Value::Float(v) => assert_eq!((v.to_bits() as i64, kind), (payload, 3)),
                            _ => panic!(),
                        }
                    }
                    ctx.has_return_value = false;
                    jit_leave_function(&mut ctx);
                    assert!(ctx.runtime_error.is_none());
                    println!("round={round} shape={shape} location={location} writes=200000 ns={elapsed}");
                }
            }
        }
    }
}
