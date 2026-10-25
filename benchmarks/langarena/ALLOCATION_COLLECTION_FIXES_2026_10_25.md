# Allocation and collection fixes — 2026-10-25

This round implements the demonstrated call/frame and global indexed-write costs
from checklist sections 3 and 4. It does **not** close every item in those sections:
fixed imported-helper setup remains expensive, and broader profiling/acceptance
work remains open. No benchmark algorithms, input sizes, checksums, or warmups
changed. No Kostya contact, upstream submission, or release publication occurred.

Runtime commits:

- `e1ac6ba`: move ordinary call arguments, avoid successful-call metadata copies,
  and reuse bounded, cleared VM frame-slot storage.
- `787498d`: preserve global array/string-key dictionary ownership across indexed
  writes, including compiler-emitted scoped `StoreVar` assignments.

Evidence: [structured measurements](allocation-collection-2026-10-25.json),
[checklist](PERFORMANCE_CHECKLIST.md), and [previous diagnosis](LOCAL_ISOLATION_2026_10_25.md).

## Build and measurement identity

The before runtime is `ea0ea3e`; the after runtime is `787498d`. The runner's label
`original` means the immediately preceding runtime in this round, **not** the
initial `f037705` performance baseline. This is not a new Goose/Rust comparison.
Both CLI binaries use normal locked default-feature release builds and Rust 1.96.0.
The final build used `cargo build --locked --release --bin kujo --lib -j 2` with
thin LTO. Builds and correctness gates finished before the paired timing runs.

| Binary | SHA-256 |
| --- | --- |
| Before | `36ae7ad967dfafbb125701ec97153a66e669c6d86e757b000e41d27a7e858505` |
| After | `6556aaba11d97e9573ec2c8670aa70a4d46a1a22878132899a061c713d2f7de4` |

Source hashes, probe/harness compiler arguments, release build log, raw outputs,
telemetry and per-case statuses are in the JSON evidence. Local executables and
working logs are under `.audit-evidence/allocation-collection-2026-10-25/`.
The Ruby runner handles host deadlines/telemetry only; diagnostic kernels remain
Kujo and allocation instrumentation remains Rust repository tooling.

## Allocation and frame storage

Ordinary bytecode calls reserve the actual bounded argument count and move those
arguments into the call. Successful arity validation borrows names and uses counts.
Legacy receiver adaptation and retry paths retain their original operands.
Malformed bytecode with an enormous argument count still reports stack underflow.

Returned/unwound frames can retain at most 32 cleared storage buffers, each with
capacity at most 256 values and bounded associated metadata. Live recursion,
callbacks and suspended generators retain independent frames. Values are dropped
before caching; metadata resets before reuse. This bounds the raw retained slot
payload to approximately 1.1 MiB in the worst case, excluding allocator overhead.
Tests check weak-reference release, buffer exclusivity, metadata reset and limits.

Ten thousand eight-argument diagnostic calls, phase 4, main-thread allocations:

| Mode | Before requests | After requests | Before requested bytes | After requested bytes |
| --- | ---: | ---: | ---: | ---: |
| VM | 240,032 | 90,036 | 40,487,466 | 11,088,586 |
| JIT enabled | 321,138 | 200,838 | 81,297,398 | 62,828,538 |

Requested bytes fall about 73% in VM and 23% with JIT enabled. These are allocation
traffic reductions, not elapsed-time speedups or retained-memory measurements.
JIT-enabled execution still includes fallback and compilation costs.

## Global collection ownership and scaling

The VM discards a loaded temporary only when the destination still has identical
backing storage and the following store resolves to that global. Captured/named
local shadowing prevents the optimization. Arrays require integer indices and
ordinary dictionaries require string keys; specialized representations retain
existing behavior. Real snapshots still force copy-on-write detachment.

Repeated writes use existing bounded weak depth evidence. Over-depth writes,
external root growth, repairing deep roots, bounds/errors and immutable bindings
remain checked. Failed or rejected conditional mutations publish no captured-write
journal entry. Reentrant index calls that replace the destination use the prior
path, preserving existing evaluation order.

For 64 global writes, requested phase-4 bytes:

| Shape / ownership | Width | Before | After |
| --- | ---: | ---: | ---: |
| Array / unique | 128 | 1,388,450 | 26,738 |
| Array / unique | 2,048 | 22,032,290 | 349,298 |
| Array / one snapshot | 2,048 | 22,032,674 | 349,682 |
| Array / fresh snapshot every write | 2,048 | 22,033,250 | 17,900,042 |
| Dictionary / unique | 128 | 2,862,418 | 72,164 |
| Dictionary / unique | 2,048 | 44,395,858 | 659,684 |
| Dictionary / one snapshot | 2,048 | 44,435,410 | 673,124 |
| Dictionary / fresh snapshot every write | 2,048 | 44,435,986 | 40,159,076 |

First-write validation/detachment can still scale with container width. Subtracting
the one-write control from the 64-write unique-owner control isolates the next
63 writes: **1,197 bytes for arrays and 28,764 bytes for dictionaries at both
widths**. Creating a new snapshot on every write legitimately retains copying.

The first prototype handled `StoreGlobal` alone. Its hand-built bytecode test
passed, but actual top-level loops emitted `StoreVar` and still copied whole
containers. That prototype was rejected; the final regression also compiles real
source loops. The retained negative evidence demonstrates why opcode-only tests
were insufficient.

Sustained dictionary removal, stack pop and queue dequeue remain linear in the
combined diagnostic. At 256 / 1,024 / 4,096 operations per collection, requested
bytes changed from 576,916 / 2,301,100 / 9,200,812 to
367,748 / 1,465,244 / 5,858,204 (about 36% lower); request counts were unchanged.
All extracted-value sums and final empty-container assertions passed. This is a
diagnostic, not a substituted Arena benchmark.

## Paired timing and resource observations

Seven alternating before/after pairs per case/runtime; lower candidate/before
ratios are better. Loop time is the final measured checkpoint minus warmup-done.
All 84 runs passed their unchanged checksum. Times below are seconds.

| Workload | Mode | Before loop median | After loop median | Paired loop ratio median (range) | Total ratio median (range) |
| --- | --- | ---: | ---: | --- | --- |
| Sort::Quick | vm | 12.261 | 10.875 | 0.910 (0.750–0.991) | 0.924 (0.749–0.983) |
| Sort::Quick | jit | 2.868 | 2.862 | 1.020 (0.954–1.281) | 1.008 (0.935–1.158) |
| Sort::Merge | vm | 34.841 | 32.363 | 0.938 (0.898–1.057) | 0.944 (0.910–1.124) |
| Sort::Merge | jit | 11.751 | 11.716 | 0.987 (0.938–1.177) | 1.002 (0.945–1.163) |
| Etc::NeuralNet | vm | 11.273 | 10.949 | 0.979 (0.951–1.063) | 1.017 (0.961–1.069) |
| Etc::NeuralNet | jit | 7.935 | 7.997 | 1.002 (0.961–1.032) | 0.994 (0.908–1.020) |

Raw before/after medians and median paired ratios answer different questions;
the JSON retains every pair and all total/loop distributions. Do not infer
statistical significance or general-machine speedups from seven local pairs.

No case has a median paired loop slowdown above 5%. Individual regressions
remain visible in the ranges; this is not a guarantee against regressions on
other inputs or hosts.

Host CPU speed-limit telemetry ranged from 44% to 55% during these runs.
Load, compression and cumulative swap counters are preserved with timestamps.
The laptop was not an isolated host; thermal/load variation can change both
absolute times and pair ratios. No result here closes clean-host acceptance.

Separate NeuralNet resource observations (one observation per build/mode,
excluded from timing statistics), peak RSS in MiB:

| Mode | Before | After |
| --- | ---: | ---: |
| vm | 17.59 | 17.63 |
| jit | 35.68 | 35.30 |

These small resource samples are observations, not a memory-regression verdict.
Frame-retention limits are additionally enforced by unit tests. The JSON also
retains seven call-microbenchmark wall-time pairs in VM/interpreter/JIT and
separate call resource runs; microbenchmarks are not Arena score claims.


## Verification and limits

- **150/150 unchanged test-size checksums** on the final standard release CLI:
  all 50 in VM, interpreter and JIT-enabled execution.
- **188 integration tests passed, one ignored**, on the final library/CLI.
  Coverage includes allocations, call/frame ownership, global indexed writes,
  runtime parity, collection/helper/import ownership, recursion, deletion/pop,
  byte mutation/concatenation, synchronous callbacks, module caches, JIT cold/deep
  callbacks and scalar depth semantics.
- **270 ownership/scaling observations passed** across VM, interpreter and JIT:
  five shapes, three ownership states, three width/operation controls, two scopes.
- **1,108 library tests passed, eight ignored** before the final `StoreVar`
  adjustment. This is broader supporting coverage, not an exact-final library
  rerun; the final integration suite includes compiled-loop scaling and depth tests.
- `cargo fmt --check`, Ruby syntax checks and `git diff --check` passed.

The integration suites were linked into one harness against the final normal
release library to avoid 16 separate expensive LTO links. It includes unchanged
test modules and uses the final CLI for subprocess tests. Exact build commands
are recorded. An interrupted earlier multi-binary build and rejected non-LTO link
are not test failures and are not counted as passing validation.

No full production sweep or new Linux/Windows acceptance was performed this round.
Historical production/platform results do not certify this revision. Allocation
traffic is main-thread probe evidence; exact native/fallback CPU-time attribution
and clean-host comparisons remain open.

## Remaining work and semantic decision

1. **Imported helper setup:** 64 read-only imported calls request 92,129 allocations
   and 13,840,564 bytes after these changes, at both array widths 128 and 2,048
   (before: 13,866,948 bytes). Width-dependent normalization is not reproduced,
   but fixed setup remains substantial. `enter_captured_environment` cloning is
   a source-backed candidate, not proven CPU attribution. Replacing environment
   materialization requires preserving captures, recursion and global writes;
   this round does not claim that fix.
2. **Existing direct-global reentrant-index disagreement:** before and after,
   VM/JIT produce `[42, 199, 7]`, while interpreter produces `[42, 9, 9]` when an
   index helper replaces the target array before the write. Nested assignments
   have a documented root-after-index contract; direct-global legacy behavior
   needs a separate semantic decision. Tests preserve each runtime's existing
   result, rather than silently changing the contract in a performance patch.
3. **Remaining metadata/collection review:** parameter-slot/name scans, specialized dictionary/field paths, iterator
   owner removal and broader byte/string conversion costs are not newly optimized.
   Existing byte and collection regression coverage passed; that is not proof
   that every possible cost is eliminated.
4. **Acceptance:** clean host, exact native/fallback attribution, broader production
   and platform verification, and competitive comparisons remain pending.
