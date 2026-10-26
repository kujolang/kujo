# Kujo performance implementation checklist

Reviewed scope: 2026-10-25. This is the human-readable tracker for
`performance-plan.json`. Check a box only when its stated gate has evidence.
Historical passes do not certify a later runtime revision. A timeout is incomplete,
and a stopped case is cancelled, not failed. Sections 1 and 2 have completed their available local diagnosis. The user has
now authorized sections 3 and 4; section 5 has not been restarted.

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
- [x] Freeze reconstructed original, verified candidate, Goose, Rust, toolchains and inputs (`local-isolation-2026-10-25.json`).
- [x] Preserve original and current candidate as baselines for the next fix; repeat freezing after each future fix.
- [ ] Establish a quiet dedicated Linux host. **Blocked: user confirmed no host.**
- [x] Record CPU/architecture, OS, RAM, toolchains and build flags for the local candidate/probe (diagnosis manifest).
- [x] Record continuous load, memory/swap and available thermal telemetry during the main local pass.
- [x] Separate builds from measured benchmark intervals.
- [x] Run seven alternating local NeuralNet comparisons in VM and JIT; clean-host comparisons remain pending.
- [x] Record local phase checkpoints, allocation phases, compile counters, loop/total time and separate peak RSS observations.
- [x] Treat rounded zero competitor timings as below reporting resolution; do not calculate zero-based speedup ratios.
- [x] Keep NeuralNet comparison algorithms/configuration/checksums identical; label modified diagnostic inputs separately.
- [x] Preserve explicit timeout, cancellation, failed and passed statuses.

Gate: repeatable measurements with exact identities and uncertainty. A noisy local
diagnostic cannot close clean-host acceptance. Removed temporary executables have
now been reconstructed from pinned sources and hashed. The preserved candidate
matches its previously verified binary hash; no current release substitution.

## 2. Diagnose dominant costs

For every row below collect CPU samples, allocation requests/requested bytes,
copy/ownership evidence, call/frame/environment costs, native/fallback counters,
setup-versus-loop boundaries, a small reproducer, and a pre-change result.
Counters alone do not establish native CPU-time coverage. Allocation traffic is
not retained memory. A diagnostic input is not an official benchmark pass.

| Priority | Workload | Diagnosis gate |
| --- | --- | --- |
| First | QuickSort | [x] local profiles/phases and call/index/ownership controls |
| First | MergeSort | [x] local profiles/phases and call/index/ownership controls |
| First | NeuralNet | [x] seven-round local regression check, phases and struct-state controls |
| First | CacheSimulation | [x] depth/hash profiles and dictionary/nested ownership scaling |
| First | GameOfLife | [x] nested-state scaling, phases and validation attribution |
| Next | Brainfuck Array | [x] dispatch/operand profiles, phases and native counters |
| Next | Brainfuck Recursion | [x] dispatch/operand/call profiles, phases and native counters |
| Next | TextRaytracer | [x] field/root profiles, phases and struct-state controls |
| Next | Small JIT workloads | [x] separate compilation/setup costs and local iteration-count payback experiment |

- [x] Capture local CPU profiles with useful symbols; preserve short/missing-window limitations.
- [x] Measure aggregate main-thread allocation count/requested bytes separately from ordinary CLI peak RSS.
- [x] Isolate user snapshots, global loaded temporaries, function locals and assertion-call effects; retain required snapshots.
- [x] Measure warmed native call controls by arity/count, helper-return scaling, named-opcode clones and disabled-debug lookups.
- [x] Preserve native/fallback event evidence and sampled helper/dispatch categories without claiming CPU-time percentages.
- [x] Separate allocation phases and container-width/operation-count scaling; retain instrumentation boundaries.
- [x] Verify representative reproducers across VM, interpreter and JIT (495 ownership observations).
- [ ] Obtain precise generated-native/fallback CPU-time attribution. **Blocked locally:** xctrace absent; DTrace cannot initialize with current privileges/SIP.
- [x] Preserve local diagnostic before results before implementation.

Gate: each optimization proposal has a reproduced bottleneck, not just a static
hypothesis. Subsequent user authorization now covers sections 3 and 4.

Available local work in sections 1–2 is complete: [local isolation report](LOCAL_ISOLATION_2026_10_25.md).
Clean-host acceptance and precise native CPU-time attribution remain open.
The earlier [priority diagnosis](PRIORITY_DIAGNOSIS_2026_10_25.md) remains historical evidence.
The reproduced global indexed-write copy/validation cost was fixed in `787498d`. Local-state snapshot semantics
passed. The earlier NeuralNet VM slowdown did not reproduce consistently locally.

## 3. Allocation and call-frame fixes (verified fixes; remaining work explicit)

Evidence: [allocation/collection report](ALLOCATION_COLLECTION_FIXES_2026_10_25.md),
`allocation-collection-2026-10-25.json`; runtime commits `e1ac6ba`, `787498d`.

- [x] Remove demonstrated avoidable hot-call allocations (10,000-call allocation control).
- [x] Reduce argument conversion/copying/temporary vectors (ordinary arguments move; legacy compatibility retained).
- [x] Reuse bounded frame storage with recursion/reentrancy isolation (32 buffers, at most 256 slots each; values cleared).
- [x] Cache ordinary parameter-slot/name metadata per immutable code object; public code edits invalidate the cache. Legacy receiver-field lookup remains unchanged.
- [x] Remove redundant mutable permission entries from imported environment snapshots: 64 calls fall from 92,129 requests / 13,840,564 bytes to 61,977 / 11,334,789.
- [ ] Reduce remaining captured value-environment materialization without changing snapshots, writes, recursion or JIT lifetimes.
- [x] Check return transfers and reusable callback state against ownership/import/callback regression suites; no new transfer defect established.
- [x] Check pooling/cache retention bounds and separately observe peak memory; no clean-host memory acceptance claim.
- [x] Test recursion, callbacks, exceptions, imports, captures and cancellation in the regression corpus (see exact revision limits in report).

Gate: the reproduced call/frame costs are reduced with bounded retention.
Section 3 remains open for fixed imported environment/setup cost. Follow-up evidence:
[imported setup and bytes report](IMPORTED_SETUP_AND_BYTES_2026_10_25.md),
`imported-setup-and-bytes-2026-10-25.json`; final 150 test-size checksums pass.

## 4. Collection and access fixes (verified global-write fix; broader review open)

- [x] Profile dictionary → array → struct access (prior diagnosis and 270 final ownership controls).
- [x] Preserve unique backing storage across repeated global array/string-dictionary mutation, including scoped compiler-emitted stores.
- [x] Preserve real snapshots through copy-on-write detachment (one snapshot and fresh-per-write controls).
- [x] Reuse valid depth evidence without repeated full-container scans in the changed path.
- [x] Verify changed-path invalidation for external growth, root replacement, deep-value rejection/repair and aliases; existing nested-path suites pass.
- [x] Check sustained dictionary deletion, stack and queue costs (256/1,024/4,096 diagnostic operations; linear scaling).
- [x] Review observable iterator snapshot costs with source mutation; first detachment is required in the tested case, so no owner was removed.
- [x] Measure representative byte conversion, UTF-8, indexing and iterator scaling; reserve bytes(array) output once, preserve ordered validation and snapshots.
- [ ] Broaden specialized representation review when profiles identify additional costs; representative controls are not universal coverage.
- [x] Add scaling checks for accidental quadratic behavior, including actual compiler-emitted loops rather than hand-built opcodes alone.
- [x] Observe peak memory separately from allocation traffic; clean-host acceptance remains pending.
- [ ] Resolve existing direct-global reentrant-index evaluation-order divergence before changing its behavior; this round preserves each runtime's baseline result.

Gate: changed global paths have better allocation scaling with ownership, errors,
bounds and depth checks intact. This does not close all collection representations
or the separate semantic decision.

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

The report records this round's evidence. These reusable gates remain requirements
for subsequent fixes; unchecked production/platform gates are not implied complete.

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

- [x] Recheck all 150 test-size combinations after these optimizations (`787498d`; allocation-collection-2026-10-25.json). Repeat for later runtime changes.
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
