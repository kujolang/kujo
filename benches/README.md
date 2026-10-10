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

## Release performance campaign

Roadmap item 3 extends the existing standalone Rust process runners with
collections/serialization, sparse dictionaries, schemas, context fitting and
ASCII/Unicode text estimation. The corpus also covers startup, import-aware
static checking/LSP, generators, captures, task admission/cancellation, channels,
promise observers and offline AI request hashing. Every runtime workload checks
an independent expected stdout and empty stderr before accepting a sample.

Build an exact source checkout with the ordinary locked release profile:

```bash
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo build --release --locked --bin kujo
```

Preserve immutable baseline/candidate binaries from their respective checkouts,
then run from this repository with Node 20+ and `rustc` on PATH:

```bash
node --test tests/release_performance_campaign.test.js
node scripts/release_performance_campaign.js \
  /absolute/baseline/kujo /absolute/candidate/kujo \
  BASELINE_40_CHARACTER_SHA CANDIDATE_40_CHARACTER_SHA \
  /absolute/new-evidence-directory
```

The output directory must not already exist. No npm install is needed. The Node
standard-library wrapper is repository-maintenance orchestration for identical
cross-platform evidence, not a Kujo application adapter. It compiles and runs the
existing Rust runners, retains raw samples, workload source, measurements,
compiler/CPU/OS metadata, binary/source checksums and statistical summaries, and
prints one receipt. Build logs/cache state accompany the evidence separately.

Runtime samples use two discarded warmups followed by eleven samples for each
of baseline, candidate and instrumented candidate, rotating their order. Static
samples use two warmups and twenty-one alternating baseline/candidate pairs.
These are warm-cache, fresh-process observations, not controlled cold-cache
results. Sustained loops amortize startup; measurement phase/call counters help
explain work, but inclusive VM time is not exclusive per-function CPU time.

Use the same binary in both slots for an A/A noise baseline. A/A differences are
not improvements. Compare candidate changes only with paired, correctness-checked
runs using the same compiler/profile/host/workloads. The summary reports min,
median, nearest-rank p90, max, mean and population standard deviation. No samples
are discarded as outliers. Do not turn hosted-runner latency into a flaky CI gate.

Unix process peak RSS and process CPU come from separate instrumented runs.
Windows may report null: null means unsupported/unavailable, never zero. Peak
RSS is not retained heap, and shallow counters are not total allocations. The
campaign does not claim syscall, network-request or descendant-process counts.
No providers or external services are called by these workloads.

The manual `release-performance` workflow builds a pinned baseline commit on
Linux x64, macOS arm64 and Windows x64. Supply an optional candidate SHA to build
and compare a second commit on each host; omit it for an A/A-only campaign.
Comparisons retain a baseline A/A run for resource and noise context. It keeps evidence
for 90 days; preserve reviewed results in a durable audit artifact before expiry.
The optional platform selector limits a retry to Linux, macOS or Windows.
Build orchestration uses the runner's native shell environment so Windows uses
native Perl for vendored OpenSSL, with a recorded Perl preflight before Cargo.
Its targeted push/pull-request job verifies statistics and evidence rejection
semantics, including failed runs and timeout cleanup. The runner has a twenty-minute
deadline and terminates its owned process tree on timeout; partial evidence is
retained and no successful receipt is emitted. To
change the corpus, review the workload's expected output, update this inventory
and the campaign report, and start a new checksum-identified measurement series.
Timing changes alone never justify updating an expected result.
