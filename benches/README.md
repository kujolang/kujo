# Rust Performance Benchmarks

This directory is intentional. It contains the Criterion suite for Kujo's
runtime, parser, module loader, AI-native pure helpers, and static-file server.
The target is wired through `Cargo.toml` and is used by the performance guide
and launch-safe regression artifacts.

Build the suite without running measurements:

```bash
cargo bench --bench v1_perf_benchmarks --no-run
```

Run one focused family:

```bash
cargo bench --bench v1_perf_benchmarks -- module_resolution --noplot
```

Criterion output under `target/criterion/` is machine-local and must not be
committed. Results are internal regression signals unless they satisfy
`docs/BENCHMARK_PUBLICATION_POLICY.md`.
