// Micro-benchmark: Direct comparison of bytecode vs JIT
// This measures the raw execution speed difference

use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::jit::JitCompiler;
use kujo::vm::VM;
use std::time::Instant;

fn main() {
    println!("=== JIT Micro-Benchmark ===\n");

    // Create a simple hot loop bytecode
    // Pseudo-code: for i in 0..1000 { i = i + 1 }
    let mut chunk = BytecodeChunk::new();

    let const_0 = chunk.add_constant(Constant::Int(0));
    let const_1 = chunk.add_constant(Constant::Int(1));
    let const_1000 = chunk.add_constant(Constant::Int(1000));

    // Initialize counter
    chunk.emit(OpCode::LoadConst(const_0)); // 0

    // Loop start (PC 1)
    let loop_start = chunk.instructions.len();

    // counter + 1
    chunk.emit(OpCode::Dup); // 1
    chunk.emit(OpCode::LoadConst(const_1)); // 2
    chunk.emit(OpCode::Add); // 3

    // Check if counter < 1000
    chunk.emit(OpCode::Dup); // 4
    chunk.emit(OpCode::LoadConst(const_1000)); // 5
    chunk.emit(OpCode::LessThan); // 6

    // Loop back if true
    let jump = chunk.emit(OpCode::JumpIfTrue(0)); // 7
    chunk.set_jump_target(jump, loop_start);

    // Return final value
    chunk.emit(OpCode::Return); // 8

    println!("Bytecode chunk: {} instructions", chunk.instructions.len());
    println!("Expected iterations: 1000\n");

    // Benchmark 1: Pure bytecode execution
    println!("Benchmark 1: Bytecode VM (100 runs)");
    let mut total_bytecode = std::time::Duration::ZERO;
    for _ in 0..100 {
        let mut vm = VM::new();
        vm.set_jit_enabled(false);
        let start = Instant::now();
        let _ = vm.execute(chunk.clone());
        total_bytecode += start.elapsed();
    }
    let avg_bytecode = total_bytecode / 100;
    println!("  Average time: {:?}", avg_bytecode);

    // Benchmark 2: Compile to native code
    println!("\nBenchmark 2: JIT Compilation");
    let mut compiler = JitCompiler::new().expect("Failed to create JIT compiler");

    let compile_start = Instant::now();
    let compiled_fn = match compiler.compile(&chunk, 0) {
        Ok(f) => {
            println!("  ✓ Compilation successful");
            f
        }
        Err(e) => {
            println!("  ✗ Compilation failed: {}", e);
            return;
        }
    };
    let compile_time = compile_start.elapsed();
    println!("  Compilation time: {:?}", compile_time);

    // Benchmark 3: Execute compiled code
    println!("\nBenchmark 3: Execute Compiled Native Code (100 runs)");
    let mut total_jit = std::time::Duration::ZERO;
    for _ in 0..100 {
        let start = Instant::now();
        unsafe {
            let _ = compiled_fn(std::ptr::null_mut());
        }
        total_jit += start.elapsed();
    }
    let avg_jit = total_jit / 100;
    println!("  Average time: {:?}", avg_jit);

    // Results
    println!("\n=== Results ===");
    println!("  Bytecode VM:     {:?}", avg_bytecode);
    println!("  Compiled code:   {:?}", avg_jit);

    let speedup = avg_bytecode.as_nanos() as f64 / avg_jit.as_nanos() as f64;
    println!("  Speedup:         {:.2}x", speedup);

    if speedup > 1.0 {
        println!("\n  ✓ JIT is {:.2}x faster than bytecode!", speedup);
    } else {
        println!("\n  Note: Compilation overhead: {:?}", compile_time);
        println!("  JIT worth it after {} runs", compile_time.as_nanos() / avg_jit.as_nanos());
    }
}
