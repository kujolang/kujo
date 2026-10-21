# LangArena follow-up verification — 2026-10-21

This follow-up fixes three measured runtime costs while preserving the unchanged
LangArena workload. **All 50 production cases were attempted; 10 completed with
matching checksums on the updated optimized runtime. The other 40 remain
unverified.** No benchmark checksum mismatch or benchmark-process runtime error
was observed. Passing
`test.json` does not prove completion of `run.json`, and a timeout does not
establish a correctness defect.

## Confirmed fixes

- `088527f`: callable bytecode uses shared storage with copy-on-write edits.
  Aliasing or calling a function no longer copies its instructions, constants and
  nested chunks. Compiler and generator edits detach first. Runtime captures
  retain their existing semantics.
- `743dbf8`: legacy named-parameter binding metadata is cached per shared code
  allocation. Calls no longer scan the function body for each parameter. Mutable
  access invalidates the cache, and cached metadata does not affect equality.
- `34b82c9`: bytecode callback bridges attach live globals without eagerly
  copying every global binding into an interpreter snapshot. Interpreter fallback
  materializes the environment before use. Cancellation, recursion limits,
  capabilities, imported helpers and fresh global values remain covered.
- `8e410cd`: production progress now includes warmup checkpoints. Workload sizes,
  RNG resets, warmup counts, measured iterations and checksums are unchanged.

The probes vary unused code or unrelated globals while executing identical work.
They are diagnostic scaling observations on a shared host, not official scores.
Storage and cache-invalidation tests verify the mechanisms independently of timing.

| Debug probe | Before, small / large | After, small / large |
| --- | ---: | ---: |
| 10,000 VM calls, 0 / 1,000 unused statements; metadata cache | 338.89 / 1068.75 ms | 218.74 / 219.03 ms |
| 2,000 VM regex callbacks, 0 / 1,000 unrelated globals | 174.04 / 1129.28 ms | 155.01 / 146.91 ms |
| Same callback probe, interpreter control | 29.39 / 28.18 ms | 29.25 / 28.44 ms |

The first row compares shared bytecode before and after metadata caching. Before
either change, the optimized callable probe took 58.49 / 1301.15 ms; these are
separate builds and observations, not a controlled comparison with the debug row.

Reproduce with [call-body probe](probes/call_body_size.kujo) and
[callback-global probe](probes/callback_globals.kujo), supplying a pinned binary
and an existing output directory. Each probe checks outputs in both runtimes.

## Verification gates

- Optimized `test.json`: all 50 checksums passed in both runtimes (100/100).
- Full library suite: 1,002 passed, 8 ignored, zero failures with two test threads.
- Targeted callback, regex, sorting, bytecode-sharing, port and runtime parity
  integration gates: 141 passed. Earlier closure/generator coverage also passed.
- VM fixtures: 150/150 passed, 11 policy skips. Dual fixtures: 150/150 passed,
  11 skips, zero fallback. Dual is VM-primary, not a second full interpreter run.
- Port contracts after warmup instrumentation: 5 passed, 1 explicit long gate
  ignored; the separate optimized long gate above ran successfully.
- Formatting, diff whitespace, minimal-feature library check and normal
  default-feature optimized build passed.

A highly parallel earlier library run exceeded an existing process-cancellation
test's two-second timing assertion on this loaded host. The isolated retry and
the entire final two-thread library run passed. No cancellation defect was
reproduced. A redundant debug interpreter all-fifty run was deliberately cancelled
after the optimized gate passed; it is not reported as a passing debug gate.
The entire repository-wide integration suite was not run.

## Production attempts

The two discovery shards allowed five minutes per case. Separate retries allowed
20 minutes for CSV, 30 minutes for regex templates, 40 minutes for NGram and two
hours for each tree variant. CSV and regex templates completed. Both trees and
NGram reached their retry limits without a final checksum. The profiling runner
exits nonzero when any child attempt is incomplete; its final assertion is not a
benchmark correctness failure. No input sizes, iteration counts or algorithms
were reduced to obtain those passes.

The current passes are Base64 encode/decode, all three JSON workloads, CSV, word
counting, log parsing, regex templates and native sorting. The earlier executable
also completed NGram; its result remains historical evidence, not a current pass.

The per-case table and raw evidence below distinguish successful final checksums
from incomplete attempts. Rows select a successful retry when one exists;
otherwise they select the last recorded attempt. Timings include all phases and
are not official benchmark scores.

| Benchmark | Result | Elapsed seconds | Last completed checkpoint / outcome |
| --- | --- | ---: | --- |
| Binarytrees::Obj | TIMEOUT | 7200.24 | run-12 |
| Binarytrees::Arena | TIMEOUT | 7200.33 | run-15 |
| Brainfuck::Array | TIMEOUT | 300.04 | warmup-done |
| Brainfuck::Recursion | TIMEOUT | 300.03 | warmup-done |
| Matmul::Single | TIMEOUT | 300.08 | prepare-done |
| Matmul::T4 | TIMEOUT | 300.08 | prepare-done |
| Matmul::T8 | TIMEOUT | 300.07 | prepare-done |
| Matmul::T16 | TIMEOUT | 300.09 | prepare-done |
| Base64::Encode | PASSED | 8.83 | checksum matched |
| Base64::Decode | PASSED | 8.61 | checksum matched |
| Json::Generate | PASSED | 32.30 | checksum matched |
| Json::ParseDom | PASSED | 47.37 | checksum matched |
| Json::ParseMapping | PASSED | 179.25 | checksum matched |
| CSV::Parse | PASSED | 568.06 | checksum matched |
| Etc::Sieve | TIMEOUT | 300.04 | warmup-5 |
| Etc::TextRaytracer | TIMEOUT | 300.04 | prepare-done |
| Etc::NeuralNet | TIMEOUT | 300.04 | warmup-34 |
| Etc::CacheSimulation | TIMEOUT | 300.06 | warmup-1 |
| Etc::GameOfLife | TIMEOUT | 300.26 | prepare-done |
| Etc::Words | PASSED | 285.00 | checksum matched |
| Etc::LogParser | PASSED | 4.42 | checksum matched |
| Template::Regex | PASSED | 1722.02 | checksum matched |
| Template::Parse | TIMEOUT | 300.04 | warmup-4 |
| Sort::Quick | TIMEOUT | 300.07 | warmup-4 |
| Sort::Merge | TIMEOUT | 300.20 | warmup-1 |
| Sort::Self | PASSED | 169.36 | checksum matched |
| Graph::BFS | TIMEOUT | 300.17 | create-done |
| Graph::DFS | TIMEOUT | 300.14 | create-done |
| Graph::AStar | TIMEOUT | 300.13 | create-done |
| Hash::SHA256 | TIMEOUT | 301.36 | prepare-done |
| Hash::CRC32 | TIMEOUT | 300.22 | prepare-done |
| Calculator::Ast | TIMEOUT | 300.22 | prepare-done |
| Calculator::Interpreter | TIMEOUT | 300.27 | create-done |
| Maze::Generator | TIMEOUT | 300.45 | create-start |
| Maze::BFS | TIMEOUT | 300.19 | create-start |
| Maze::AStar | TIMEOUT | 300.14 | create-start |
| CLBG::Fannkuchredux | TIMEOUT | 300.16 | warmup-1 |
| CLBG::Mandelbrot | TIMEOUT | 300.22 | prepare-done |
| CLBG::Nbody | TIMEOUT | 300.08 | warmup-1 |
| CLBG::Spectralnorm | TIMEOUT | 300.13 | prepare-done |
| Compress::BWTEncode | TIMEOUT | 300.09 | prepare-done |
| Compress::BWTDecode | TIMEOUT | 300.54 | create-done |
| Compress::HuffEncode | TIMEOUT | 300.10 | prepare-done |
| Compress::HuffDecode | TIMEOUT | 300.10 | create-done |
| Compress::ArithEncode | TIMEOUT | 300.14 | prepare-done |
| Compress::ArithDecode | TIMEOUT | 301.08 | prepare-done |
| Compress::LZWEncode | TIMEOUT | 300.24 | warmup-5 |
| Compress::LZWDecode | TIMEOUT | 300.36 | create-done |
| Distance::Jaro | TIMEOUT | 300.83 | warmup-1 |
| Distance::NGram | TIMEOUT | 2400.09 | run-10 |

Raw attempts, binary/configuration hashes, probe outputs and historical retries:
[follow-up evidence](production-followup-2026-10-21.json).

## Remaining gates

- Linux checksum coverage remains unverified. An immutable source snapshot at
  `8e410cd` was used for a build attempt in `rust:1.90-bookworm` with one CPU and
  3 GiB of memory,
  using `cargo build --locked --release --no-default-features --features
  runtime-network --bin kujo`. The dependency build reached its 90-minute limit
  (exit 124) before producing a binary; neither checksum runtime was executed.
  No compiler error was reported before the timeout. The first invocation's
  missing CLI-required `runtime-network` feature was corrected; that invocation
  mistake is not a runtime defect. This attempted minimal CLI build would not
  establish default-feature release packaging coverage even if it had passed.
- Complete the 40 still-unverified production checksums on the current runtime.
  Several numerical, recursive, graph and compression workloads did not finish
  preparation or warmup within the discovery window. Their partial progress is
  not a passing result, and these observations alone do not identify a specific
  language defect to fix. Further throughput profiling needs an uncontended host.
- Measure production memory and controlled elapsed time. These shared-host runs
  cannot establish resource limits or competitive performance.
- Verify the full production configuration in the interpreter if full-size
  parity is required; this session verified both runtimes against `test.json`.
- Finish official driver/timing protocol integration, language registration and
  distribution packaging before an upstream submission.
- Obtain maintainer review of typed-JSON mapping through a DOM, indexed shared
  graph representations and the matrix worker-pool lifecycle. Checksum parity
  does not decide acceptance of those representation choices.

There is no justified claim here that all remaining work is a Kujo language bug,
or that every production benchmark is ready. The three reproduced runtime costs
above are fixed; the remaining verification and submission gates remain open.

## Provenance and limits

Runtime source: `34b82c909ebbf7a50c19e9835b562ac700a58959`.
Port source: `8e410cd1483c2b96d682415c6e04f0b425d22fa4`.
Optimized macOS executable SHA-256:
`fde0f6c0ab999c97349c4a1ee05db6497532fcfa7765a5a654f32d08ff1976e2`.
The build uses the repository's normal default features, thin LTO and one codegen
unit, with Rust 1.96.0 on macOS x86_64.

Both configuration files remain byte-for-byte identical to pinned upstream
`3c9f84ddaeec834c0631747cb789e8cfd602f993`. No benchmark algorithm was substituted.
The shared host ran concurrent workloads and builds and reported thermal CPU
limiting. Elapsed time includes startup, preparation, warmup, all iterations and
the final checksum. These runs establish completion where successful, not
competitive performance. Timeout or cancellation leaves a case unverified.

The [earlier production report](PRODUCTION_VERIFICATION.md) remains historical
evidence for the earlier executable. Its passes cannot be represented as fresh
verification of this executable. Full production interpreter coverage, official
driver/timing integration and maintainer acceptance remain separate gates.
The JSON mapping, graph representation and worker-pool choices documented in
the [README](README.md#representation-choices-requiring-review-before-submission)
still need review before submission. No Kostya contact or upstream submission
occurred.
