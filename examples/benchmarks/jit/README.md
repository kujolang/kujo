# JIT Performance Benchmarks

This directory contains benchmarks for testing Kujo's JIT compilation performance (Phase 4E).

## Benchmark Programs

### 1. `arithmetic_intensive.kujo`
Tests pure integer arithmetic performance. Should trigger Int type specialization.
```bash
cargo run --release -- run examples/benchmarks/jit/arithmetic_intensive.kujo
```

### 2. `variable_heavy.kujo`
Tests performance with many local variables (8 variables). Exercises type profiling and guard generation.
```bash
cargo run --release -- run examples/benchmarks/jit/variable_heavy.kujo
```

### 3. `loop_nested.kujo`
Tests nested loop performance. Inner loop should get JIT-compiled.
```bash
cargo run --release -- run examples/benchmarks/jit/loop_nested.kujo
```

### 4. `comparison_specialized.kujo`
Pure Int operations - ideal case for specialization.
```bash
cargo run --release -- run examples/benchmarks/jit/comparison_specialized.kujo
```

### 5. `comparison_generic.kujo`
Mixed type operations - forces generic code paths.
```bash
cargo run --release -- run examples/benchmarks/jit/comparison_generic.kujo
```

### 6. `run_all.kujo`
Runs all benchmarks and compares results.
```bash
cargo run --release -- run examples/benchmarks/jit/run_all.kujo
```

## Expected Performance Characteristics

**Phase 4 (Type Specialization) Goals:**
- 2-3x faster than Phase 3 baseline for variable-heavy code
- Specialized Int operations should use direct i64 instructions
- Guard overhead should be minimal (46µs per guard measured)
- Type profiling overhead should be negligible (11.5M observations/sec)

**Comparison:**
- Specialized code (pure Int): Fastest - uses optimized i64 paths
- Generic code (mixed types): Slower - must handle multiple types
- Difference should demonstrate Phase 4 specialization benefits

## How JIT Works in Kujo

1. **Interpretation** (first 100 executions): Code runs in tree-walking interpreter
2. **Type Profiling** (during interpretation): System tracks variable types
3. **Compilation Threshold** (100+ executions): JIT compiler activates
4. **Specialized Compilation**: Generates optimized code for observed types
5. **Guard Insertion**: Type checks ensure assumptions remain valid
6. **Native Execution**: JIT-compiled code runs at near-native speed

## Measuring Speedup

To compare JIT vs interpreter:
```bash
# With JIT (default)
cargo run --release -- run examples/benchmarks/jit/arithmetic_intensive.kujo

# Without JIT (interpreter only)
# TODO: Add --no-jit flag to disable JIT compilation
```

## Phase 4E Goals

- ✅ Benchmark infrastructure complete
- ✅ Real-world benchmark programs created
- ⏳ Validate 2-3x speedup claim
- ⏳ Document actual performance gains
- ⏳ Compare specialized vs generic paths
