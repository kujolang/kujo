# Runtime instrumentation overhead evidence

Internal regression evidence, not a public performance guarantee. See
`docs/BENCHMARK_PUBLICATION_POLICY.md`.

- Host: Intel Core i7-9750H 2.60 GHz, macOS 26.6.2 (25G83), x86_64.
- Toolchain: rustc 1.96.0 (ac68faa20 2026-05-25).
- Baseline source: immutable `git archive cd6d2ea`, built in `/tmp/kujo-next-baseline`.
- Candidate runtime source: `bdf634f`; same Cargo.lock and default features.
- `debug-overhead.json`: standard dev profile, unoptimized with debug information.
- Harness: `scripts/runtime_measurement_bench.rs`, compiled with `rustc -O`.
- Each workload: one discarded warmup per mode, then 11 measured samples per mode;
  order rotates baseline/disabled/enabled to reduce drift bias.
- Every sample asserts successful execution and identical stdout/stderr.
- Timings include child startup, parsing/compilation, runtime shutdown and, in
  enabled mode, exclusive report creation and file sync. They do not isolate
  counters from export cost. Captures/generators and async tasks use real execution.
- Host power/thermal state and unrelated background load were not controlled.
  These samples describe this host/run only. Compare paired observations and
  dispersion, not just a ratio of medians. No JIT performance claim is made.

Reproduce using two executables built with the same profile/toolchain:

```bash
rustc -O scripts/runtime_measurement_bench.rs -o /tmp/kujo-measure-bench
/tmp/kujo-measure-bench /path/to/baseline-kujo /path/to/candidate-kujo > results.json
```

The host harness is Rust because it compares independent runtime executables
without requiring either candidate's timing or process API to supply the baseline.
