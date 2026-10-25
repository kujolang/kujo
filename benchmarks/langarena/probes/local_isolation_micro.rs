// Direct mechanism controls. Setup is outside accounting, cloned values are
// consumed with black_box. No runtime implementation changes or shortcut kernels.
fn micro_controls() {
    use kujo::{bytecode::OpCode, interpreter::Value};
    use std::{hint::black_box, sync::Arc, time::Instant};
    for n in [128usize, 512, 2048] {
        for mode in [
            "scalar_opcode",
            "named_opcode",
            "unique_array",
            "one_snapshot",
            "snapshot_each",
            "environment",
            "cached_boolean_control",
        ] {
            let scalar = OpCode::LoadConst(0);
            let named = OpCode::LoadVar("diagnostic_binding".into());
            let mut array = Arc::new(vec![Value::Int(0); n]);
            let snapshot = if mode == "one_snapshot" { Some(array.clone()) } else { None };
            ALLOCATIONS.with(|v| v.set(0));
            REQUESTED_BYTES.with(|v| v.set(0));
            TRACK.with(|v| v.set(true));
            let started = Instant::now();
            let operations = if mode == "environment" || mode == "cached_boolean_control" {
                1_000_000
            } else {
                128
            };
            for i in 0..operations {
                match mode {
                    "scalar_opcode" => {
                        black_box(black_box(&scalar).clone());
                    }
                    "named_opcode" => {
                        black_box(black_box(&named).clone());
                    }
                    "environment" => {
                        let _ = black_box(std::env::var(black_box("DEBUG_JIT_CALL")));
                    }
                    "cached_boolean_control" => {
                        black_box(black_box(false));
                    }
                    _ => {
                        let each = if mode == "snapshot_each" { Some(array.clone()) } else { None };
                        Arc::make_mut(&mut array)[i % n] = Value::Int(i as i64 + 1);
                        if let Some(old) = each {
                            assert!(matches!(old[i % n], Value::Int(0)));
                        }
                    }
                }
            }
            let elapsed = started.elapsed().as_nanos();
            TRACK.with(|v| v.set(false));
            if let Some(old) = snapshot {
                assert!(matches!(old[0], Value::Int(0)));
            }
            println!("MICRO mode={mode} size={n} operations={operations} requests={} bytes={} elapsed_ns={elapsed}", ALLOCATIONS.with(Cell::get), REQUESTED_BYTES.with(Cell::get));
        }
    }
}
