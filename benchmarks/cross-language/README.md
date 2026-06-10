# Cross-Language Performance Benchmarks

This directory contains comprehensive performance benchmarks comparing **Kujo**, **Python**, and **Go** across common programming tasks.

## Purpose

Show how Kujo's JIT compilation and optimizations compare to:
- **Python** - Popular interpreted language with similar syntax
- **Go** - Statically compiled language known for speed

## Benchmark Suite

The suite tests 8 different performance characteristics:

| Benchmark | Tests | Key Insight |
|-----------|-------|-------------|
| 1. Fibonacci Recursive | Function call overhead | How expensive are function calls? |
| 2. Fibonacci Iterative | Loop performance | How fast are loops? |
| 3. Array Sum | Array iteration | How efficiently can we iterate? |
| 4. Hash Map Operations | Dictionary/map performance | How fast are hash lookups? |
| 5. String Concatenation | String operations | How expensive is string building? |
| 6. Nested Loops | Loop optimization | Can the compiler optimize nested loops? |
| 7. Array Building | Dynamic array construction | How efficient is memory allocation? |
| 8. Object Creation | Struct/object allocation | How fast can we create objects? |

## Running the Benchmarks

### Prerequisites

- **Kujo** built in release mode: `cargo build --release`
- **Python 3** installed
- **Go** installed

### Run All Benchmarks

```bash
cd benchmarks/cross-language
./run_benchmarks.sh
```

This will:
1. Compile the Go benchmark
2. Run Kujo, Python, and Go versions sequentially
3. Save results to `results/benchmark_TIMESTAMP.txt`
4. Display summary

### Run Individual Benchmarks

```bash
# Kujo
../../target/release/kujo bench.kujo

# Python
python3 bench.py

# Go
go run bench.go
```

## Understanding Results

### What to Expect

**Kujo** should show:
- ✅ **Fast function calls** - JIT compilation optimizes hot paths
- ✅ **Efficient loops** - LLVM-optimized loop code
- ✅ **Good hash map performance** - Native Rust HashMap backend
- ⚠️ **Variable string concat** - Depends on runtime optimization

**Python** typically:
- ❌ Slow recursive functions (interpreted overhead)
- ❌ Slower loops (bytecode interpretation)
- ✅ Decent hash maps (optimized C implementation)

**Go** typically:
- ✅✅ Very fast compiled code
- ✅✅ Efficient memory allocation
- ✅✅ Fast loops and function calls

### Typical Performance Profile

```
Fibonacci Recursive (n=30):
  Go:     ~5-10ms    (compiled, fastest)
  Kujo:   ~50-100ms  (JIT-compiled)
  Python: ~500-700ms (interpreted, slowest)

Array Sum (1M elements):
  Go:     ~2-5ms     (compiled, cache-friendly)
  Kujo:   ~5-15ms    (JIT-optimized)
  Python: ~30-50ms   (interpreted loop)

Hash Map Operations (100k items):
  Go:     ~10-20ms   (compiled)
  Kujo:   ~20-40ms   (native HashMap)
  Python: ~15-30ms   (optimized dict)
```

## Key Takeaways

### Where Kujo Excels

1. **Much faster than Python** - JIT compilation provides 5-10x speedup
2. **Good optimization** - LLVM backend generates efficient code
3. **Dynamic + Fast** - Get Python-like flexibility with better performance

### Where Kujo Lags

1. **Not as fast as compiled Go** - JIT warmup and dynamic typing overhead
2. **First-run overhead** - JIT compilation takes time before optimization kicks in

### The Sweet Spot

Kujo is ideal for:
- Scripts that need better performance than Python
- Applications where compilation time matters
- Use cases that benefit from REPL + good performance
- Projects that want expressiveness without sacrificing too much speed

## Implementation Notes

All three implementations are functionally equivalent:
- Same algorithms
- Same test sizes
- Same operations
- Fair comparison (no language-specific tricks)

### Fairness Considerations

- **No NumPy/pandas** in Python (would be unfair - C extensions)
- **No cgo** in Go (keep it pure Go)
- **Kujo with JIT** enabled (default mode)

## Files

- `bench.kujo` - Kujo implementation
- `bench.py` - Python implementation
- `bench.go` - Go implementation
- `run_benchmarks.sh` - Automated runner
- `results/` - Timestamped benchmark results

## Adding New Benchmarks

To add a new benchmark:

1. Implement in all three languages (`bench.kujo`, `bench.py`, `bench.go`)
2. Keep implementations equivalent
3. Add to this README's table
4. Document what it tests

## Contributing

When running benchmarks for comparison:
- Use release mode for Kujo (`cargo build --release`)
- Run multiple times to account for variance
- Report system specs (CPU, RAM, OS)
- Include timestamps and versions

## Version Info

- Kujo: v0.10.0 (release hardening + architecture cleanup)
- Python: 3.x
- Go: 1.x

---

**Last Updated:** 2026-02-18
