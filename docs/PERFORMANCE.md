# Kujo Performance Guide

This guide covers performance characteristics, profiling tools, and optimization strategies for the Kujo programming language.

## Table of Contents

1. [Performance Overview](#performance-overview)
2. [Execution Modes](#execution-modes)
3. [Profiling Tools](#profiling-tools)
4. [Benchmarking](#benchmarking)
5. [Cross-Language Comparisons](#cross-language-comparisons)
6. [Optimization Tips](#optimization-tips)
7. [JIT Compilation](#jit-compilation)

---

## Performance Overview

Kujo offers three execution modes with different performance characteristics:

| Mode | Speed vs Interpreter | Compilation Time | Use Case |
|------|---------------------|------------------|----------|
| **Interpreter** | 1x (baseline) | None | Debugging, development |
| **VM** | 10-50x | Instant | Production scripts |
| **JIT** | 100-500x+ | First run only | Long-running, computation-heavy |

### Performance Targets

- **CPU-Bound**: 2-5x slower than Go, 2-10x faster than Python
- **I/O-Bound**: Near-native performance (bottleneck is I/O, not language)
- **Memory**: Similar to Python/Node.js (Arc-based GC)

---

## Execution Modes

### 1. Tree-Walking Interpreter

The original execution mode. Best for:
- Development and debugging
- Scripts that run once
- Testing

**Usage:**
```bash
kujo run script.kujo --interpreter
```

**Characteristics:**
- Slowest execution
- No compilation overhead
- Easy to debug
- Full language feature support

### 2. Bytecode VM (Default)

Compiles AST to bytecode, then executes. Best for:
- Production scripts
- Moderate computation
- General use

**Usage:**
```bash
kujo run script.kujo  # VM is default
```

**Characteristics:**
- 10-50x faster than interpreter
- Instant compilation (<1ms typically)
- Low memory overhead
- Recommended for most use cases

### 3. JIT Compilation

Hot-path detection triggers native compilation. Best for:
- Long-running services
- Computation-heavy workloads
- Performance-critical code

**Usage:**
```bash
# JIT activates automatically after 100 iterations
# No special flags needed
```

**Characteristics:**
- 100-500x faster for arithmetic-heavy code
- Compilation happens on first hot-path detection
- Type specialization for common types
- Guard checks for type stability

---

## Profiling Tools

Kujo includes built-in profiling to identify performance bottlenecks.

### Basic Profiling

```bash
kujo profile script.kujo
```

**Output:**
```
=== Performance Profile Report ===

CPU Profile:
  Total Time: 2.543s
  Samples: 1250

  Top Hot Functions:
    1. calculate_primes          1.234s (48.5%)
    2. fibonacci                 0.456s (17.9%)
    3. process_data              0.321s (12.6%)

Memory Profile:
  Peak Memory: 45.23 MB
  Current Memory: 12.45 MB
  Total Allocations: 125043
  Total Deallocations: 124950

  Top Allocation Hotspots:
    1. array_creation            45230 allocs
    2. string_concatenation      23410 allocs
    3. dict_operations           12340 allocs

JIT Statistics:
  Functions Compiled: 3
  Recompilations: 0
  Total Compile Time: 0.045s
  Cache Hit Rate: 95.2%
  Guard Success Rate: 98.7%
===================================
```

### Advanced Profiling Options

```bash
# Disable specific profiling categories
kujo profile script.kujo --no-cpu
kujo profile script.kujo --no-memory
kujo profile script.kujo --no-jit

# Generate flamegraph data
kujo profile script.kujo --flamegraph profile.txt

# Visualize with flamegraph.pl (install from GitHub)
flamegraph.pl profile.txt > flamegraph.svg
open flamegraph.svg
```

### Flamegraph Workflow

1. **Install flamegraph tools:**
   ```bash
   git clone https://github.com/brendangregg/FlameGraph
   export PATH=$PATH:$(pwd)/FlameGraph
   ```

2. **Profile your script:**
   ```bash
   kujo profile compute_heavy.kujo --flamegraph profile.txt
   ```

3. **Generate SVG:**
   ```bash
   flamegraph.pl profile.txt > flamegraph.svg
   ```

4. **View in browser:**
   ```bash
   open flamegraph.svg  # macOS
   xdg-open flamegraph.svg  # Linux
   ```

---

## Benchmarking

Kujo includes a built-in benchmarking framework.

### Running Benchmarks

```bash
# Run all benchmarks in directory
kujo bench examples/benchmarks/

# Run specific benchmark
kujo bench fibonacci.kujo

# Custom iterations and warmup
kujo bench fibonacci.kujo -i 20 -w 5
```

### Benchmark Output

```
========================================
Kujo Performance Benchmarks
========================================

Comparing execution modes:
  Interpreter: Tree-walking AST interpreter
  VM: Bytecode virtual machine
  JIT: Just-in-time native compilation
========================================

fibonacci
  Interpreter:  2.543s  (1.00x)
  VM:           0.156s  (16.30x faster)
  JIT:          0.008s  (317.88x faster) 🚀

higher_order
  Interpreter:  1.234s  (1.00x)
  VM:           0.089s  (13.87x faster)
  JIT:          0.012s  (102.83x faster) 🚀

========================================
Summary:
  Total benchmarks: 8
  VM speedup: 10-20x
  JIT speedup: 100-300x
========================================
```

### Creating Custom Benchmarks

Create a `.kujo` file that demonstrates the operation to benchmark:

```kujo
# fibonacci_bench.kujo

func fibonacci(n) {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

# Benchmark will measure this execution
let result := fibonacci(30)
print(result)
```

---

## Cross-Language Comparisons

Compare Kujo against Go, Python, and Node.js.

### Running Comparisons

```bash
chmod +x examples/benchmarks/compare_languages.sh
./examples/benchmarks/compare_languages.sh
```

### Example Results

#### Fibonacci (Recursive, N=30)

| Language | Time | Relative Speed |
|----------|------|----------------|
| Go | 0.045s | 1.00x (baseline) |
| Kujo (JIT) | 0.156s | 0.29x (2-3x slower) ✅ |
| Node.js (V8) | 0.234s | 0.19x |
| Kujo (VM) | 1.234s | 0.04x |
| Python | 4.567s | 0.01x |

#### Array Operations (100K elements)

| Language | Map | Filter | Reduce | Total |
|----------|-----|--------|--------|-------|
| Go | 2ms | 3ms | 1ms | **6ms** |
| Node.js | 15ms | 18ms | 8ms | **41ms** |
| Kujo (JIT) | 23ms | 28ms | 12ms | **63ms** |
| Kujo (VM) | 145ms | 167ms | 89ms | **401ms** |
| Python | 234ms | 267ms | 145ms | **646ms** |

### Performance Targets vs Reality

- ✅ **Go**: 2-5x slower (Target met)
- ✅ **Python**: 2-10x faster (Target met)
- ✅ **Node.js**: Competitive (Target met)

---

## Optimization Tips

### 1. Let JIT Warm Up

The JIT compiler activates after 100 iterations. Ensure hot loops run enough:

```kujo
# ❌ Not enough iterations for JIT
for i in range(50) {
    expensive_calculation()
}

# ✅ JIT will activate
for i in range(200) {
    expensive_calculation()
}
```

### 2. Avoid Type Mixing in Hot Loops

Type guards add overhead. Keep types consistent:

```kujo
# ❌ Type mixing defeats JIT optimization
x := 0
for i in range(1000) {
    x := x + i  # int
    checkpoint := i / 100
    if checkpoint * 100 == i {
        x := x + 0.5  # suddenly float! Guard fails
    }
}

# ✅ Consistent types enable specialization
y := 0
for j in range(1000) {
    y := y + j  # always int
}
```

### 3. Hoist Invariant Calculations

Move calculations outside loops:

```kujo
# ❌ Recalculates every iteration
for i in range(1000) {
    let multiplier := expensive_function()
    result := i * multiplier
}

# ✅ Calculate once
let multiplier := expensive_function()
for i in range(1000) {
    result := i * multiplier
}
```

### 4. Use Native Functions

Built-in functions are optimized in Rust:

```kujo
# ❌ Manual implementation
func sum_array(arr) {
    let total := 0
    for x in arr {
        total := total + x
    }
    return total
}

# ✅ Use built-in
let total := sum(arr)
```

### 5. Preallocate Collections

Avoid repeated resizing:

```kujo
# ❌ Multiple reallocations
let arr := []
for i in range(10000) {
    arr.push(i)
}

# ✅ Preallocate if possible (feature planned)
# let arr := Array.with_capacity(10000)
```

### 6. Profile Before Optimizing

Don't guess - measure:

```bash
# Find the actual bottleneck
kujo profile script.kujo --flamegraph profile.txt
```

---

## JIT Compilation

### How It Works

1. **Hot Path Detection**: After 100 iterations, functions are marked "hot"
2. **Type Profiling**: VM tracks types of variables
3. **Compilation**: Hot functions compile to native code with Cranelift
4. **Guard Insertion**: Type checks ensure assumptions hold
5. **Execution**: Native code runs 100-500x faster
6. **Deoptimization**: If guards fail, fall back to VM

### Type Specialization

The JIT generates optimized code based on observed types:

```kujo
func calculate(x, y) {
    return x * y + x / y
}

# After profiling sees Int + Int:
# JIT generates: (x * y) as i64 + (x / y) as i64

# If later called with Float:
# Guard fails, deoptimizes to VM
```

### JIT Thresholds

| Setting | Value | Purpose |
|---------|-------|---------|
| Hot threshold | 100 iterations | When to compile |
| Specialization samples | 50 | Minimum observations |
| Guard failure rate | 10% | When to despecialize |

### Monitoring JIT

Check JIT statistics in profile output:

```bash
kujo profile script.kujo

# JIT Statistics section shows:
# - Functions compiled
# - Cache hit rate
# - Guard success rate
```

**Healthy JIT metrics:**
- Cache hit rate: >90%
- Guard success rate: >95%
- Few recompilations

**Unhealthy metrics indicate:**
- Type instability (mixing types)
- Not enough iterations (hot threshold not reached)
- Complex control flow (defeats optimization)

---

## Troubleshooting Performance

### Problem: Slower than expected

**Diagnosis:**
```bash
kujo profile script.kujo
```

**Common causes:**
- Not enough iterations for JIT
- Type mixing in hot paths
- I/O-bound, not CPU-bound
- Using interpreter mode

**Solutions:**
- Increase loop iterations
- Keep types consistent
- Profile to find bottlenecks
- Use VM mode (default)

### Problem: High memory usage

**Diagnosis:**
```bash
kujo profile script.kujo --memory
```

**Common causes:**
- String concatenation in loops
- Large array/dict allocations
- Closure captures
- Leaked references

**Solutions:**
- Use string builders (feature planned)
- Preallocate collections
- Minimize closure scope
- Check allocation hotspots

### Problem: JIT not activating

**Check:**
1. Hot threshold reached? (100+ iterations)
2. VM mode enabled? (not --interpreter)
3. Profile shows "Functions Compiled: 0"?

**Solutions:**
- Increase loop count
- Remove --interpreter flag
- Check for exceptions in loops

---

## Further Reading

- [ROADMAP.md](../ROADMAP.md) - Performance milestone details
- [JIT Implementation](../docs/JIT_INTERNALS.md) - Technical deep dive
- [Benchmarking Framework](../src/benchmarks/README.md) - Implementation details

---

## LSP Latency Guardrails (v0.13.0)

Kujo tracks baseline latency guardrails for key editor-loop operations using representative source samples.

Guarded operations:

- completion (`textDocument/completion` equivalent helper path)
- diagnostics (`textDocument/publishDiagnostics` equivalent helper path)
- hover (`textDocument/hover` equivalent helper path)

Guardrail test:

```bash
cargo test --test lsp_latency_guardrails
```

The guardrail test intentionally uses conservative thresholds to detect major regressions while remaining stable on loaded CI hosts.

Related reliability tests:

- `cargo test lsp_server::tests::cancelled_request_returns_cancelled_error`
- `cargo test lsp_server::tests::timeout_returns_timeout_error_shape`
- `cargo test lsp_server::tests::non_object_json_message_does_not_panic_or_emit_response`

---

*Last Updated: April 2026 (v0.13.0 cross-IDE foundation)*
