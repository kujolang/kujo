# Real-World Benchmarks for Kujo

This directory contains comprehensive real-world benchmarks for evaluating Kujo's performance across different workload types.

## Benchmark Suite

### 1. JSON Parsing & Serialization (`json_parsing.kujo`)

Tests JSON operations with various data sizes and complexities:

- **Serialization**: Convert Kujo data structures to JSON strings
- **Parsing**: Parse JSON strings into Kujo objects
- **Round-trip**: Full serialize → parse → serialize → parse cycle
- **Nested structures**: Deeply nested object handling (10 levels)

**Test sizes**: 10, 50, 100, 500 user records with nested profile data

**Metrics**:
- Time (ms)
- Throughput (operations/sec)
- JSON string length
- Data validation

### 2. File I/O Operations (`file_io.kujo`)

Comprehensive file system operations benchmark:

- **Sequential write**: Write files of various sizes (10-500 KB)
- **Sequential read**: Read files and measure throughput
- **Append operations**: Test append performance (10-100 operations)
- **Line-by-line processing**: Simulate log file parsing
- **File copy**: Read + write operations combined
- **Multiple small files**: Create/read many small files (10-50 files)

**Metrics**:
- Time (ms)
- Throughput (KB/s)
- Operations per second
- File sizes and counts

### 3. Sorting Algorithms (`sorting_algorithms.kujo`)

Compares three sorting implementations:

- **QuickSort**: Divide-and-conquer, O(n log n) average
- **MergeSort**: Stable sort, O(n log n) guaranteed
- **Built-in**: Optimized native implementation

**Test patterns**:
- Random data
- Already sorted (QuickSort worst case)
- Reverse sorted
- Nearly sorted (10% swaps)

**Test sizes**: 50, 100, 200, 500 elements

**Metrics**:
- Time (ms)
- Items per second
- Correctness validation

### 4. String Processing (`string_processing.kujo`)

Seven categories of string operations:

1. **Concatenation**: Direct vs array join strategies
2. **Searching**: Pattern matching in large text
3. **Splitting & Parsing**: CSV-like data processing
4. **Transformations**: Case, trim, replace, camel/snake/kebab
5. **Pattern Matching**: Log file parsing simulation
6. **Validation**: Email validation logic
7. **Substring Operations**: Substring extraction and indexOf

**Metrics**:
- Operations per second
- Throughput (lines/sec, searches/sec, validations/sec)
- Strategy comparisons (speedup factors)

## Running Benchmarks

### Run all real-world benchmarks:
```bash
cargo run --release -- run examples/benchmarks/json_parsing.kujo
cargo run --release -- run examples/benchmarks/file_io.kujo
cargo run --release -- run examples/benchmarks/sorting_algorithms.kujo
cargo run --release -- run examples/benchmarks/string_processing.kujo
```

### Using the benchmark command:
```bash
# Run with custom iterations and warmup
cargo run --release -- bench examples/benchmarks/json_parsing.kujo -i 10 -w 3
```

### Run all benchmarks in directory:
```bash
for file in examples/benchmarks/*.kujo; do
    echo "Running $file..."
    cargo run --release -- run "$file"
    echo ""
done
```

## Benchmark Results

Results will show:
- Operation type
- Data size/iterations
- Time elapsed (ms)
- Throughput metrics
- Validation/correctness checks

Example output:
```
=== JSON Parsing & Serialization Benchmark ===

1. JSON Serialization Benchmark
  Size: 10 users -> 15 ms, 2847 bytes
  Size: 50 users -> 68 ms, 14235 bytes
  ...

2. JSON Parsing Benchmark
  Size: 10 users -> 12 ms, 10 parsed
  ...
```

## Performance Characteristics

### Expected Performance:

- **JSON**: ~100-500 records/sec for complex nested structures
- **File I/O**: 
  - Sequential: 1-10 MB/s (depends on disk)
  - Append: 100-1000 ops/sec
  - Small files: 50-200 ops/sec
- **Sorting**:
  - QuickSort: ~5,000-20,000 items/sec
  - MergeSort: ~3,000-15,000 items/sec
  - Built-in: Variable (optimized)
- **Strings**:
  - Concatenation: Array join 2-5x faster than direct
  - Searching: 10,000-50,000 searches/sec
  - Parsing: 1,000-5,000 lines/sec
  - Transformations: 5,000-20,000 ops/sec

### Interpreter vs VM vs JIT:

These benchmarks can be run with different execution modes:

- `--interpreter`: Tree-walking interpreter (baseline)
- Default (VM mode): 10-50x faster than interpreter
- With JIT enabled: 100-500x faster for numeric-heavy workloads

## Adding New Benchmarks

When adding new real-world benchmarks:

1. **Create focused scenarios**: Test specific real-world use cases
2. **Vary data sizes**: Test with small, medium, and large inputs
3. **Measure correctly**: Use `time_ms()` for timing
4. **Validate results**: Verify correctness of operations
5. **Report metrics**: Show time, throughput, and data characteristics
6. **Document patterns**: Explain what the benchmark tests

Example structure:
```kujo
func benchmark_operation(size) {
    # Setup
    data := create_test_data(size)
    
    # Measure
    start := time_ms()
    result := perform_operation(data)
    elapsed := time_ms() - start
    
    # Validate
    if not is_valid(result) {
        throw "Validation failed"
    }
    
    # Return metrics
    return {
        "operation": "name",
        "size": size,
        "time_ms": elapsed,
        "throughput": calculate_throughput(size, elapsed)
    }
}
```

## Notes

- All benchmarks create temporary files in `./tmp` directory
- File I/O benchmark cleans up test files after completion
- Benchmarks validate correctness to ensure implementations work properly
- Results may vary based on hardware and system load
- Run benchmarks multiple times for consistent results
- Use `--release` mode for accurate performance measurements

## Future Additions

Planned real-world benchmarks:

- Database operations (SQLite/Postgres/MySQL)
- HTTP client/server performance
- Concurrent/async workloads
- Cryptographic operations
- Image/media processing
- Regex performance
- Network I/O

## Profiling

For detailed performance analysis:

1. CPU profiling: Identify hot functions
2. Memory profiling: Find allocation bottlenecks
3. JIT statistics: Compilation hit rates
4. Instruction cache: Locality analysis

See Phase 6 roadmap for profiling tool integration plans.
