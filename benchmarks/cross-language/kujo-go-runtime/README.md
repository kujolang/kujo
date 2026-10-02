# Kujo Cross-Language Runtime Microbenchmarks

This suite replaces the ad hoc single-file prime comparison with equivalent
function-local workloads that provide three narrow signals:

- `startup`: process startup and trivial output;
- `prime_count`: trial-division prime counting with function-local state;
- `integer_mix`: a deterministic integer-arithmetic loop with function-local
  state.

These are local regression and profiling inputs, not broad language rankings.
Each runner checks exact output before accepting timings and keeps raw results in
the repository's ignored `benchmarks/cross-language/results/` directory.
The original matrix covers Kujo VM/JIT, Go, optimized Rust, Python, and PHP;
the leaderboard runner adds Zero and Bend without replacing that baseline.

## Run

Requirements: Rust/Cargo, Go, Python 3, PHP, Hyperfine, `jq`, and either
`shasum` or `sha256sum`.

From the repository root:

```bash
bash benchmarks/cross-language/kujo-go-runtime/run.sh
```

For the seven-language leaderboard, install Zero 0.3.4 and Bend 2.0.34, then
run:

```bash
bash benchmarks/cross-language/kujo-go-runtime/run-seven-language.sh
```

That runner compiles the Kujo fixtures with `kujo-aot` and compares the pinned
native executables against optimized native Go, Rust, Zero, and Bend
executables plus Python and PHP. Kujo compilation is recorded separately and
excluded from execution timing. It uses five startup
warmups with 25 measured runs and three compute warmups with 15 measured runs
by default. `BENCH_WARMUP`, `BENCH_RUNS`, `STARTUP_WARMUP`, and `STARTUP_RUNS`
override those sample counts. The generated summary includes the median matrix,
Kujo-to-language ratios, and a ranked leaderboard for every workload.

For the focused four-language latency comparison using the lean trusted
`kujo-run` launcher:

```bash
bash benchmarks/cross-language/kujo-go-runtime/run-four-language.sh
```

That runner compares the three runtime workloads across Kujo JIT, Go, Python,
and PHP. It builds `kujo-run` with only `runtime-jit`, records exact-output
checks before timing, and stores raw distributions plus pinned build metadata.
That latency-focused build excludes network transports and the OS credential
store in addition to the database, image, PDF, and archive feature families.

The default campaign uses one warmup and five measured runs for compile and
compute tests, five instrumented Kujo phase runs, plus three warmups and fifteen
measured startup runs. Override them when iterating locally:

```bash
BENCH_WARMUP=1 BENCH_RUNS=3 PHASE_RUNS=3 STARTUP_RUNS=10 \
  bash benchmarks/cross-language/kujo-go-runtime/run.sh
```

Hyperfine launches benchmark commands directly with `--shell=none`, avoiding
shell-startup noise that can materially distort these short-lived processes.

## Kujo native AOT scope

`kujo-aot` is a production command for allocation-free scalar programs. It
supports integer and boolean locals, direct scalar function calls, branches,
loops, checked integer arithmetic, and `print`/`to_string` output. It emits a
freestanding optimized executable on macOS or Linux for arm64 and x86_64.
Heap values, imports, async/generator functions, indirect calls, and other
unsupported syntax fail compilation with an explicit error; there is no silent
VM fallback. Compile a supported program with:

```bash
cargo run --release --no-default-features --bin kujo-aot -- program.kujo -o program
```

The runner builds Kujo with `cargo build --release --locked`, copies that exact
binary into the result bundle, builds Go binaries with the standard optimized
compiler defaults and `-trimpath`, and builds Rust binaries with `rustc -C
opt-level=3`, stripped debug data, and one codegen unit. It records all runtime
and compiler versions, the Kujo commit and dirty state, native artifact hashes,
host data, commands, and raw Hyperfine JSON/Markdown.

## Measurement boundaries

- Startup measures equivalent no-work programs in all languages/runtimes.
- Compile/check measures `kujo check --quiet`, fresh optimized Go and Rust
  native builds, Python bytecode compilation, and PHP syntax checking. These
  operations produce different artifacts and are reported side by side for
  transparency, not as directly equivalent compiler throughput.
- Run measures `kujo run`, `kujo run --jit`, already-built Go and Rust binaries,
  Python with bytecode-cache writes disabled, and PHP CLI. Kujo, Python, and PHP
  rows include process startup and source loading/compilation; the Kujo JIT row
  also includes native compilation of hot regions. Use the startup and compile
  distributions to understand those fixed costs; do not subtract medians and
  present the result as a precise execution-only measurement.
- Kujo internal phases use the bounded runtime-measurement collector to report
  repeated source parse, bytecode compile, VM setup, VM execution, and nested
  JIT compilation distributions without subtracting unrelated medians. These
  phases intentionally exclude process and CLI startup.

Every computational fixture keeps its state inside one function and uses the
same loop bounds, constants, integer operations, output, and expected-result
check. This controls obvious source-level differences without claiming that
the language implementations or compilation models are identical.

Bend 2 currently exposes a native `U32` but no native 64-bit integer type. Its
`integer_mix` fixture therefore uses overflow-safe modular multiplication to
evaluate the same recurrence and exact result. That fixture is semantically
equivalent but performs more primitive operations than the other versions, so
interpret its Bend timing with that limitation in mind.

Any public performance claim must also satisfy
[`docs/BENCHMARK_PUBLICATION_POLICY.md`](../../../docs/BENCHMARK_PUBLICATION_POLICY.md).
