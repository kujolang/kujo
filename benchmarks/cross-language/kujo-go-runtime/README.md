# Kujo and Go Runtime Microbenchmarks

This suite replaces the ad hoc single-file prime comparison with equivalent
function-local workloads that provide three narrow signals:

- `startup`: process startup and trivial output;
- `prime_count`: trial-division prime counting with function-local state;
- `integer_mix`: a deterministic integer-arithmetic loop with function-local
  state.

These are local regression and profiling inputs, not broad language rankings.
The runner checks exact output before accepting timings and keeps raw results in
the repository's ignored `benchmarks/cross-language/results/` directory.

## Run

Requirements: Rust/Cargo, Go, Hyperfine, `jq`, and either `shasum` or
`sha256sum`.

From the repository root:

```bash
bash benchmarks/cross-language/kujo-go-runtime/run.sh
```

The default campaign uses one warmup and five measured runs for compile and
compute tests, plus three warmups and fifteen measured startup runs. Override
them when iterating locally:

```bash
BENCH_WARMUP=1 BENCH_RUNS=3 STARTUP_RUNS=10 \
  bash benchmarks/cross-language/kujo-go-runtime/run.sh
```

The runner builds Kujo with `cargo build --release --locked`, copies that exact
binary into the result bundle, builds optimized Go binaries with `-trimpath`,
and records versions, the Kujo commit, dirty state, artifact hashes, host data,
commands, and raw Hyperfine JSON/Markdown.

## Measurement boundaries

- Startup measures a no-work program in each runtime.
- Compile measures `kujo check --quiet` (parse plus bytecode compilation) and a
  fresh `go build` into the result bundle. Go produces a reusable native binary;
  Kujo currently does not emit a reusable bytecode artifact, so the operations
  are reported separately rather than treated as equivalent.
- Run measures `kujo run` and an already-built Go binary. The Kujo command still
  includes process startup and source-to-bytecode compilation. Use the startup
  and compile distributions to understand those fixed costs; do not subtract
  medians and present the result as a precise execution-only measurement.

Any public performance claim must also satisfy
[`docs/BENCHMARK_PUBLICATION_POLICY.md`](../../../docs/BENCHMARK_PUBLICATION_POLICY.md).
