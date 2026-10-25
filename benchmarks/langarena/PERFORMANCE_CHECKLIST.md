# Kujo performance implementation checklist

Reviewed scope: 2026-10-25. This is the human-readable tracker for
`performance-plan.json`. Check a box only when its stated gate has evidence.
Historical passes do not certify a later runtime revision. A timeout is incomplete,
and a stopped case is cancelled, not failed. Sections 1 and 2 are authorized now;
runtime optimization in sections 3–5 has not been restarted.

## Completed foundation

- [x] Local port and checksum runner for all 50 benchmarks.
- [x] Assignment, nested collection, append/removal/pop and ownership fixes.
- [x] Reduced call/argument allocation and native-root bookkeeping.
- [x] Callback reuse, module-cache ownership and native code-identity fixes.
- [x] JIT arithmetic/type/error/assignment guards and broader native operations.
- [x] Worker JIT policy and cancellation propagation fixes.
- [x] Latest frozen release `ea0ea3e`: 150/150 test-size checksums across VM,
  interpreter and JIT (`nonleaf-release-2026-10-25.json`).
- [x] Latest relevant gate: 1,105 library passes, eight ignored; 74 targeted
  integration passes (`nonleaf-callback-reuse-2026-10-25.json`).
- [x] One six-variant all-50 comparison (`nonleaf-comparison-all50-2026-10-25.json`).
- [x] Linux/Windows 21-runtime/3-static release corpus and pinned-source audit
  (`nonleaf-platform-partial-2026-10-25.json`). This is not LangArena production.
- [x] Partial production preserved: 13 JIT passes, five timeouts, GameOfLife
  cancelled; 31 JIT cases and both other modes not reached
  (`nonleaf-production-stopped-2026-10-25.json`).

## 1. Reliable measurement foundation

- [x] Reconcile the machine ledger with the latest committed evidence (diagnostic_review_2026_10_25).
- [ ] Freeze original baseline, candidate, Goose, Rust, toolchains and inputs.
- [ ] Preserve original and immediate-predecessor baselines for each future fix.
- [ ] Establish a quiet dedicated Linux host. **Blocked: user confirmed no host.**
- [x] Record CPU/architecture, OS, RAM, toolchains and build flags for the local candidate/probe (diagnosis manifest).
- [x] Record continuous load, memory/swap and available thermal telemetry during the main local pass.
- [x] Separate builds from measured benchmark intervals.
- [ ] Run seven alternating/rotating focused comparison rounds.
- [ ] Separate startup/setup, compilation/warmup, measured loop, total time and RSS.
- [ ] Treat rounded zero competitor timings as below reporting resolution.
- [ ] Keep compared algorithms, inputs, iterations, warmups and checksums identical.
- [x] Preserve explicit timeout, cancellation, failed and passed statuses.

Gate: repeatable measurements with exact identities and uncertainty. A noisy local
diagnostic cannot close clean-host acceptance. Old `/tmp` baseline/competitor
binaries are absent at this restart; reconstruct them from pins, never substitute
a current release silently. The surviving candidate must match its saved hash.

## 2. Diagnose dominant costs

For every row below collect CPU samples, allocation requests/requested bytes,
copy/ownership evidence, call/frame/environment costs, native/fallback counters,
setup-versus-loop boundaries, a small reproducer, and a pre-change result.
Counters alone do not establish native CPU-time coverage. Allocation traffic is
not retained memory. A diagnostic input is not an official benchmark pass.

| Priority | Workload | Diagnosis gate |
| --- | --- | --- |
| First | QuickSort | [ ] comparisons/calls/indexing/allocation/ownership |
| First | MergeSort | [ ] comparisons/calls/indexing/allocation/ownership |
| First | NeuralNet | [ ] repeat VM regression; attribute dominant operations |
| First | CacheSimulation | [ ] dictionary/nested mutation/aliases/validation |
| First | GameOfLife | [ ] nested reads/writes/iteration/validation |
| Next | Brainfuck Array | [ ] dispatch/bytes/fallback |
| Next | Brainfuck Recursion | [ ] dispatch/bytes/recursive calls/fallback |
| Next | TextRaytracer | [ ] structs/vector helpers/numerics/calls |
| Next | Small JIT workloads | [ ] compilation and transition break-even |

- [x] Capture local CPU profiles with useful symbols; preserve short/missing-window limitations.
- [x] Measure aggregate main-thread allocation count/requested bytes separately from ordinary CLI peak RSS.
- [ ] Identify owners behind repeated copies; distinguish required snapshots.
- [ ] Attribute call-frame setup, argument/return transfer and environment lookup.
- [ ] Measure native execution/fallback without inventing coverage percentages.
- [ ] Separate one-time setup from per-element/per-iteration work.
- [ ] Create representative small reproducers and verify their mechanism.
- [x] Preserve local diagnostic before results before implementation.

Gate: each optimization proposal has a reproduced bottleneck, not just a static
hypothesis. No runtime optimization is authorized by completing this diagnosis.

Local diagnostic results and remaining attribution gates: [priority diagnosis](PRIORITY_DIAGNOSIS_2026_10_25.md). Neither section 1 nor section 2 is fully closed.

## 3. Allocation and call-frame fixes (not started in this round)

- [ ] Remove demonstrated avoidable hot-call allocations.
- [ ] Reduce argument conversion/copying/temporary vectors.
- [ ] Reuse bounded frame storage with recursion/reentrancy isolation.
- [ ] Avoid repeated immutable metadata scans.
- [ ] Reduce environment materialization and imported-value normalization.
- [ ] Transfer returns without unnecessary owners or copies.
- [ ] Retain reusable callback state without stale globals.
- [ ] Check pooling/cache retention and peak memory.
- [ ] Test recursion, callbacks, exceptions, imports, captures and cancellation.

Gate: lower measured costs with semantics intact and no unbounded retention.

## 4. Collection and access fixes (not started in this round)

- [ ] Profile dictionary → array → struct access.
- [ ] Preserve unique backing storage across repeated mutation.
- [ ] Preserve real snapshots through copy-on-write detachment.
- [ ] Reuse valid depth evidence without repeated full-container scans.
- [ ] Verify cache invalidation for every relevant external mutation path.
- [ ] Check sustained dictionary deletion, stack and queue costs.
- [ ] Remove unnecessary iterator owners only where safe.
- [ ] Review byte/string conversion and indexing costs.
- [ ] Add scaling checks for accidental quadratic behavior.
- [ ] Check peak memory as well as allocation traffic.

Gate: demonstrably better scaling/cost with alias, error, bounds and depth contracts.

## 5. Compiled execution (not started in this round)

- [ ] Refresh per-benchmark native execution/fallback evidence.
- [ ] Identify hot unsupported operations and expensive transitions.
- [ ] Extend compiled execution for demonstrated hot paths.
- [ ] Reduce measured guard/dispatch overhead safely.
- [ ] Preserve integer/Boolean/float/error semantics.
- [ ] Verify branches, recursion, imports, captures and unwinding.
- [ ] Tune JIT admission against small-workload compilation costs.
- [ ] Measure compilation time, code memory and cache lifetimes.
- [ ] Keep JIT opt-in pending reliability/portability acceptance.
- [ ] Consider architecture changes only if profiles justify them.

Gate: measured speed improvements, not just additional compilable functions.

## 6. Per-fix regression gate

- [ ] Reproduce the original issue before modifying it.
- [ ] Add a test that detects its return.
- [ ] Test VM, interpreter and JIT.
- [ ] Test unique ownership and shared snapshots.
- [ ] Test relevant errors, cancellation and capabilities.
- [ ] Run targeted tests, formatting and required builds.
- [ ] Broaden regression tests according to the affected runtime surface.
- [ ] Compare matching release builds before/after.
- [ ] Investigate repeatable slowdowns above 5%.
- [ ] Reject unreviewed tradeoffs that damage other workloads.
- [ ] Commit/push each verified logical change separately.
- [ ] Resolve parallel-callback global-write semantics against the canonical
  contract before changing behavior; escalate a real ambiguity for user review.

## 7. All-50 verification

Each family below requires exact input/checksum, build identity, status, elapsed
time, resource evidence and limitations for every case and runtime.

| Benchmark | Production VM | Production interpreter | Production JIT |
| --- | --- | --- | --- |
| Binarytrees::Obj | [ ] | [ ] | [ ] |
| Binarytrees::Arena | [ ] | [ ] | [ ] |
| Brainfuck::Array | [ ] | [ ] | [ ] |
| Brainfuck::Recursion | [ ] | [ ] | [ ] |
| Matmul::Single | [ ] | [ ] | [ ] |
| Matmul::T4 | [ ] | [ ] | [ ] |
| Matmul::T8 | [ ] | [ ] | [ ] |
| Matmul::T16 | [ ] | [ ] | [ ] |
| Base64::Encode | [ ] | [ ] | [ ] |
| Base64::Decode | [ ] | [ ] | [ ] |
| Json::Generate | [ ] | [ ] | [ ] |
| Json::ParseDom | [ ] | [ ] | [ ] |
| Json::ParseMapping | [ ] | [ ] | [ ] |
| CSV::Parse | [ ] | [ ] | [ ] |
| Etc::Sieve | [ ] | [ ] | [ ] |
| Etc::TextRaytracer | [ ] | [ ] | [ ] |
| Etc::NeuralNet | [ ] | [ ] | [ ] |
| Etc::CacheSimulation | [ ] | [ ] | [ ] |
| Etc::GameOfLife | [ ] | [ ] | [ ] |
| Etc::Words | [ ] | [ ] | [ ] |
| Etc::LogParser | [ ] | [ ] | [ ] |
| Template::Regex | [ ] | [ ] | [ ] |
| Template::Parse | [ ] | [ ] | [ ] |
| Sort::Quick | [ ] | [ ] | [ ] |
| Sort::Merge | [ ] | [ ] | [ ] |
| Sort::Self | [ ] | [ ] | [ ] |
| Graph::BFS | [ ] | [ ] | [ ] |
| Graph::DFS | [ ] | [ ] | [ ] |
| Graph::AStar | [ ] | [ ] | [ ] |
| Hash::SHA256 | [ ] | [ ] | [ ] |
| Hash::CRC32 | [ ] | [ ] | [ ] |
| Calculator::Ast | [ ] | [ ] | [ ] |
| Calculator::Interpreter | [ ] | [ ] | [ ] |
| Maze::Generator | [ ] | [ ] | [ ] |
| Maze::BFS | [ ] | [ ] | [ ] |
| Maze::AStar | [ ] | [ ] | [ ] |
| CLBG::Fannkuchredux | [ ] | [ ] | [ ] |
| CLBG::Mandelbrot | [ ] | [ ] | [ ] |
| CLBG::Nbody | [ ] | [ ] | [ ] |
| CLBG::Spectralnorm | [ ] | [ ] | [ ] |
| Compress::BWTEncode | [ ] | [ ] | [ ] |
| Compress::BWTDecode | [ ] | [ ] | [ ] |
| Compress::HuffEncode | [ ] | [ ] | [ ] |
| Compress::HuffDecode | [ ] | [ ] | [ ] |
| Compress::ArithEncode | [ ] | [ ] | [ ] |
| Compress::ArithDecode | [ ] | [ ] | [ ] |
| Compress::LZWEncode | [ ] | [ ] | [ ] |
| Compress::LZWDecode | [ ] | [ ] | [ ] |
| Distance::Jaro | [ ] | [ ] | [ ] |
| Distance::NGram | [ ] | [ ] | [ ] |

- [ ] Recheck all 150 test-size combinations after optimizations.
- [ ] Complete unchanged production in VM, interpreter and JIT separately.
- [ ] Preserve timeout/failure/cancellation distinctions.
- [ ] Use final-candidate evidence rather than substitute old passes.
- [ ] Investigate resource growth/scaling failures.
- [ ] Retain auditable raw outputs and identities for every pass.

Gate: all 150 production checksums verified; no reduced-input substitutions.

## 8. Competitive and platform acceptance

- [ ] Repeat Goose/Rust comparisons on the same clean host.
- [ ] Report every case, loss and unresolved measurement.
- [ ] Report distributions/variability rather than best timings.
- [ ] Include startup and memory costs.
- [ ] Track targets against the faster comparator: 2×, 1.25×, repeatable wins.
- [ ] Treat targets as goals, never guaranteed outcomes.
- [ ] Finish macOS and rerun affected Linux/Windows gates.
- [ ] Verify relevant supported architectures/features.
- [ ] Verify release artifacts and local runner integration.
- [ ] Prepare remaining maintainer questions without contacting Kostya.
- [ ] Obtain separate authorization for upstream submission or publication.

## Execution policy

Profile and reproduce the slow cases before another broad production sweep.
Use targeted production checks after meaningful fixes, and full sweeps at
milestones. No benchmark-specific native kernels, altered algorithms, weakened
checks, or performance claims based on interrupted/failed samples.
