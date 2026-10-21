# LangArena production verification — 2026-10-21

**7/50 unchanged production workloads completed with matching checksums in
the final optimized VM. The other 43 remain unverified after the 30-second
discovery deadline. This is not a completed production-suite pass.** No checksum
mismatch or runtime error was observed in the completed production workloads.
A timeout is an incomplete attempt, not evidence of a language correctness bug.

Two confirmed costs were fixed and pushed:

- `eebf9fe`: binding-level dynamic string self-appends now reuse unaliased storage
  in both runtimes. Retained snapshots, self-appends, immutability and RHS
  replacement semantics remain covered by regression tests.
- `376b36f`: bounded weak metadata caches stable character counts; ASCII indexed
  reads and substring extraction no longer rescan the entire input. Unicode
  semantics are unchanged. Unicode random indexed reads still walk the prefix.

## Method and limits

The port and both input files remain pinned to upstream
`3c9f84ddaeec834c0631747cb789e8cfd602f993`. No production sizes, iteration counts,
warmups, expected checksums or algorithms were reduced or replaced. Each case
ran in an isolated child process. The runner checkpoints results and exits
nonzero if any selected case is incomplete or fails.

Both baseline and final sweeps used two shards and a 30-second discovery limit.
The baseline completed four cases; its CSV and JSON mapping retries also exceeded
300 seconds. The final sweep completed four cases; separate 900-second retries
completed JSON DOM parsing, JSON mapping and CSV parsing, bringing the final
optimized total to seven distinct cases. The final debug binary additionally
completed full production JSON mapping, CSV and log parsing; these are supplemental
checks, not additional distinct benchmarks or optimized performance results.

The host was an Intel i7-9750H Mac with 16 GiB RAM, macOS 26.6.2 and Rust 1.96.0.
The final executable used the normal default-feature release build, including
its configured thin LTO. It was pinned before running; SHA-256:
`7039301d78a5ed0a6d8d2536e79c36a2ff025b94d6f9b4011cc98f9c2671ec9d`.
Runtime source: `376b36f4d4eec0ee774a35884cab61c21675dd33`. The baseline executable was retained from the
preceding checksum-verification work; its exact hash is recorded in the raw data.

Times include process startup, instrumentation, preparation, warmup, measured
iterations and final checksum. The shared machine also ran other work. These
single observations are **not official LangArena scores or controlled speedup
measurements**. Peak memory was not measured. Full production coverage of the
interpreter, Linux/Docker validation, upstream timing integration and representation
acceptance remain outside these results.

## Confirmed string costs

The committed probes use identical work at 16,000 and 64,000 iterations. Append
builds 512,000 and 2,048,000 bytes; scan visits every character of an ASCII string.
All printed lengths/counts were checked. Both final runtimes show approximately
fourfold time for fourfold work. Storage-identity and cache-invalidation unit tests
verify the mechanism independently of noisy wall-clock measurements.

Milliseconds (baseline and final are optimized executables):

| Probe / runtime | Baseline 16k | Baseline 64k | Fixed 16k | Fixed 64k |
| --- | ---: | ---: | ---: | ---: |
| append / vm | 1717.78 | 111985.25 | 15.24 | 60.85 |
| append / interpreter | 844.15 | 142168.99 | 12.94 | 49.17 |
| scan / vm | 2858.23 | 21289.25 | 57.54 | 216.40 |
| scan / interpreter | 1295.02 | 12504.88 | 41.35 | 162.96 |

Full production `Etc::LogParser` previously exhausted the 30-second limit during
preparation; the fixed optimized run completed all configured work and its checksum
in 3.58 seconds. The exact observations, inputs and output are retained below.

## All 50 production cases

PASS requires the expected final checksum and successful process exit. INCOMPLETE
means the entire configured workload was attempted but did not finish within the
30-second discovery window. Its last checkpoint does not prove that later phases
would pass. Rows use the successful longer retry when available.

| Benchmark | Result | Elapsed seconds | Last completed checkpoint / outcome |
| --- | --- | ---: | --- |
| Binarytrees::Obj | INCOMPLETE | 30.36 | prepare-done |
| Binarytrees::Arena | INCOMPLETE | 30.31 | prepare-done |
| Brainfuck::Array | INCOMPLETE | 30.04 | prepare-done |
| Brainfuck::Recursion | INCOMPLETE | 30.05 | prepare-done |
| Matmul::Single | INCOMPLETE | 30.27 | prepare-done |
| Matmul::T4 | INCOMPLETE | 30.20 | prepare-done |
| Matmul::T8 | INCOMPLETE | 30.55 | prepare-done |
| Matmul::T16 | INCOMPLETE | 30.22 | prepare-done |
| Base64::Encode | PASS | 4.64 | checksum matched |
| Base64::Decode | PASS | 4.57 | checksum matched |
| Json::Generate | PASS | 21.39 | checksum matched |
| Json::ParseDom | PASS | 34.47 | checksum matched |
| Json::ParseMapping | PASS | 138.80 | checksum matched |
| CSV::Parse | PASS | 401.84 | checksum matched |
| Etc::Sieve | INCOMPLETE | 30.13 | prepare-done |
| Etc::TextRaytracer | INCOMPLETE | 30.02 | prepare-done |
| Etc::NeuralNet | INCOMPLETE | 30.03 | prepare-done |
| Etc::CacheSimulation | INCOMPLETE | 30.03 | prepare-done |
| Etc::GameOfLife | INCOMPLETE | 30.29 | create-done |
| Etc::Words | INCOMPLETE | 30.03 | prepare-done |
| Etc::LogParser | PASS | 3.58 | checksum matched |
| Template::Regex | INCOMPLETE | 30.05 | prepare-done |
| Template::Parse | INCOMPLETE | 30.06 | prepare-done |
| Sort::Quick | INCOMPLETE | 30.07 | prepare-done |
| Sort::Merge | INCOMPLETE | 30.09 | prepare-done |
| Sort::Self | INCOMPLETE | 30.10 | run-1 |
| Graph::BFS | INCOMPLETE | 30.15 | create-done |
| Graph::DFS | INCOMPLETE | 30.12 | create-done |
| Graph::AStar | INCOMPLETE | 30.12 | create-done |
| Hash::SHA256 | INCOMPLETE | 30.03 | prepare-done |
| Hash::CRC32 | INCOMPLETE | 30.03 | prepare-done |
| Calculator::Ast | INCOMPLETE | 30.07 | prepare-done |
| Calculator::Interpreter | INCOMPLETE | 30.06 | create-done |
| Maze::Generator | INCOMPLETE | 30.20 | create-start |
| Maze::BFS | INCOMPLETE | 30.21 | create-start |
| Maze::AStar | INCOMPLETE | 30.18 | create-start |
| CLBG::Fannkuchredux | INCOMPLETE | 30.03 | prepare-done |
| CLBG::Mandelbrot | INCOMPLETE | 30.02 | prepare-done |
| CLBG::Nbody | INCOMPLETE | 30.02 | prepare-done |
| CLBG::Spectralnorm | INCOMPLETE | 30.03 | prepare-done |
| Compress::BWTEncode | INCOMPLETE | 30.06 | prepare-done |
| Compress::BWTDecode | INCOMPLETE | 30.35 | create-done |
| Compress::HuffEncode | INCOMPLETE | 30.02 | prepare-done |
| Compress::HuffDecode | INCOMPLETE | 30.02 | create-done |
| Compress::ArithEncode | INCOMPLETE | 30.02 | prepare-done |
| Compress::ArithDecode | INCOMPLETE | 30.02 | prepare-done |
| Compress::LZWEncode | INCOMPLETE | 30.03 | prepare-done |
| Compress::LZWDecode | INCOMPLETE | 30.04 | create-done |
| Distance::Jaro | INCOMPLETE | 30.03 | prepare-done |
| Distance::NGram | INCOMPLETE | 30.04 | prepare-done |

The remaining 43 rows need longer full runs and focused profiling before
production readiness can be claimed. This sweep does not establish that each
requires a language change; it establishes that their full-size checksums, total
runtime and resource usage are still unverified.

## Verification gates

- 135 targeted integration tests passed: diagnostics, port/profiler contracts,
  string append/access regressions, and VM/interpreter parity.
- 19 string-focused unit tests passed, including capacity reuse and bounded weak
  cache invalidation without retaining payloads.
- The explicit all-fifty integration-test executable passed 100 checks against the
  final optimized binary (50 VM + 50 interpreter, upstream `test.json`), in 431.36s.
  It was invoked with `KUJO_LANGARENA_BINARY=/tmp/kujo-production-final-bin` and
  `--ignored --nocapture`. This is the same gate documented in the port README.
- VM fixture gate: 150/150 passed, 11 policy skips.
- Dual fixture gate: 150/150 passed, 11 policy skips, zero interpreter fallback.
- `cargo check --no-default-features --lib`, `cargo fmt --check`, and the normal
  default-feature optimized build passed.
- The complete repository-wide `cargo test` suite was not run; validation was
  targeted to the runtime changes plus the entire benchmark checksum suite.

## Evidence and reproduction

- [Baseline attempts](production-baseline-2026-10-21.json)
- [Final attempts, extended runs, hashes and probe measurements](production-fixed-2026-10-21.json)
- [Append probe](probes/string_append.kujo)
- [ASCII scan probe](probes/string_scan.kujo)
- [Profiling runner and commands](README.md#production-diagnostics)

For an individual remaining case, use the unchanged `run.json`, select its exact
name, and choose a longer child deadline. Preserve the pinned executable and
configuration hash. Do not substitute `test.json` results for production completion.

No Kostya contact or upstream submission occurred.
