# Cross-Language Benchmark Inputs

This directory contains only the four source inputs used by Kujo's built-in
benchmark commands. They are maintained as local regression and profiling
tools, not as published v1.0 performance claims.

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
