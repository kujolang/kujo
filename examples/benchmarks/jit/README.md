# JIT Performance Benchmarks

This directory contains historical benchmarks for testing Kujo's JIT
compilation behavior.

Status: local JIT exploration fixtures. These files are not v1.0 launch
benchmark claims; see `../../../docs/BENCHMARK_PUBLICATION_POLICY.md` and
`../../../docs/PERFORMANCE.md`.

## Benchmark Programs

### 1. `arithmetic_intensive.kujo`
Tests pure integer arithmetic performance. Should trigger Int type specialization.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/arithmetic_intensive.kujo
```

### 2. `variable_heavy.kujo`
Tests performance with many local variables (8 variables). Exercises type profiling and guard generation.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/variable_heavy.kujo
```

### 3. `loop_nested.kujo`
Tests nested loop performance. Inner loop should get JIT-compiled.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/loop_nested.kujo
```

### 4. `comparison_specialized.kujo`
Pure Int operations - ideal case for specialization.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/comparison_specialized.kujo
```

### 5. `comparison_generic.kujo`
Mixed type operations - forces generic code paths.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/comparison_generic.kujo
```

### 6. `run_all.kujo`
Runs all benchmarks and compares results.
```bash
cargo run --release -- run --jit examples/benchmarks/jit/run_all.kujo
```

## Local Performance Characteristics

These goals are historical local-development signals, not public v1.0
performance claims:

- Compare against the same commit, command, and machine.
- Specialized Int operations should use direct i64 instructions
- Guard overhead and type profiling overhead should be measured before being
  cited.

**Comparison:**
- Specialized code (pure Int): expected to exercise optimized i64 paths when
  the workload is JIT-compatible
- Generic code (mixed types): expected to exercise fallback or generic paths
- Difference should demonstrate Phase 4 specialization benefits

## How JIT Works in Kujo v1.0

JIT execution is experimental and opt-in with `kujo run --jit`. Unsupported
bytecode surfaces fall back to VM execution with deterministic messaging. Do not
infer that a benchmark used JIT unless the command, output, and compatibility
surface confirm it.

## Measuring Speedup

To compare JIT vs interpreter:
```bash
# With JIT opt-in
cargo run --release -- run --jit examples/benchmarks/jit/arithmetic_intensive.kujo

# Default VM path
cargo run --release -- run examples/benchmarks/jit/arithmetic_intensive.kujo

# Interpreter fallback path
cargo run --release -- run --interpreter examples/benchmarks/jit/arithmetic_intensive.kujo
```

## Phase 4E Goals

- ✅ Benchmark infrastructure complete
- ✅ Real-world benchmark programs created
- ⏳ Validate local performance behavior with current commands
- ⏳ Document actual measured behavior with artifacts before making claims
- ⏳ Compare specialized vs generic paths
