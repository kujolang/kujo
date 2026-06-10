// Simpler benchmark: Pure arithmetic without loops
// This tests whether pure stack operations can be compiled

use kujo::bytecode::{BytecodeChunk, Constant, OpCode};
use kujo::jit::JitCompiler;
use kujo::vm::VM;
use std::time::Instant;

fn main() {
    println!("=== Simple Arithmetic JIT Test ===\n");

    // Test: 5 + 3 * 2 = 11
    let mut chunk = BytecodeChunk::new();
    let const_5 = chunk.add_constant(Constant::Int(5));
    let const_3 = chunk.add_constant(Constant::Int(3));
    let const_2 = chunk.add_constant(Constant::Int(2));

    chunk.emit(OpCode::LoadConst(const_5)); // 5
    chunk.emit(OpCode::LoadConst(const_3)); // 5, 3
    chunk.emit(OpCode::LoadConst(const_2)); // 5, 3, 2
    chunk.emit(OpCode::Mul); // 5, 6
    chunk.emit(OpCode::Add); // 11
    chunk.emit(OpCode::Return);

    println!("Expression: 5 + 3 * 2");
    println!("Expected: 11\n");

    // Test bytecode VM
    println!("Test 1: Bytecode VM");
    let mut vm = VM::new();
    vm.set_jit_enabled(false);
    let result1 = vm.execute(chunk.clone()).expect("VM execution failed");
    println!("  Result: {:?}", result1);

    // Benchmark bytecode VM
    let start = Instant::now();
    for _ in 0..10000 {
        let mut vm = VM::new();
        vm.set_jit_enabled(false);
        let _ = vm.execute(chunk.clone());
    }
    let bytecode_time = start.elapsed();
    println!("  10000 runs: {:?}", bytecode_time);

    // Test JIT compilation
    println!("\nTest 2: JIT Compilation");
    let mut compiler = JitCompiler::new().expect("Failed to create JIT compiler");

    match compiler.compile(&chunk, 0) {
        Ok(compiled_fn) => {
            println!("  ✓ Compilation successful");

            // Try to execute it
            println!("\nTest 3: Execute Compiled Code");
            let mut ctx = kujo::jit::VMContext::new(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            let result = unsafe { compiled_fn(&mut ctx as *mut kujo::jit::VMContext) };
            println!("  Return code: {}", result);

            // Benchmark compiled code
            let start = Instant::now();
            for _ in 0..10000 {
                let mut ctx = kujo::jit::VMContext::new(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                unsafe {
                    compiled_fn(&mut ctx as *mut kujo::jit::VMContext);
                }
            }
            let jit_time = start.elapsed();
            println!("  10000 runs: {:?}", jit_time);

            // Calculate speedup
            println!("\n=== Results ===");
            println!("  Bytecode VM: {:?}", bytecode_time);
            println!("  JIT:         {:?}", jit_time);
            let speedup = bytecode_time.as_nanos() as f64 / jit_time.as_nanos() as f64;
            println!("  Speedup:     {:.2}x", speedup);
        }
        Err(e) => {
            println!("  ✗ Compilation failed: {}", e);
        }
    }
}
