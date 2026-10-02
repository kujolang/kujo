# Cross-Language Benchmark Inputs

The four top-level source inputs are used by Kujo's built-in benchmark
commands. They are maintained as local regression and profiling tools, not as
published v1.0 performance claims.

The historically named [`kujo-go-runtime/`](kujo-go-runtime/) subdirectory
contains a separate, reviewable Kujo, Go, Rust, Python, and PHP microbenchmark
suite. It uses equivalent function-local workloads, pins native binaries,
toolchain versions, and the source revision in each result bundle, validates
exact output before timing, and reports startup, compile/check, and end-to-end
run distributions separately.

Run from the repository root with an optimized build:

```bash
cargo run --release -- bench-cross
cargo run --release -- bench-ssg --warmup-runs 1 --runs 3
cargo run --release -- bench-ssg --warmup-runs 1 --runs 3 --compare-python
```

Requirements:

- Rust/Cargo and the repository's locked dependencies
- Python 3 for `bench-cross` and `bench-ssg --compare-python`
- enough local temporary-disk capacity for the 10,000-file SSG fixture

The command harnesses validate machine-readable metrics and checksums before
reporting comparisons. SSG scratch data is removed after each run; use
`--tmp-dir <path>` to select its temporary root.

Do not commit raw timing output. Results vary by hardware, operating system,
filesystem, toolchain, background load, and build profile. Any public claim
must satisfy `docs/BENCHMARK_PUBLICATION_POLICY.md`.
