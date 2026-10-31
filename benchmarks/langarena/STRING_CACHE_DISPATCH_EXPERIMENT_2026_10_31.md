# String-cache dispatch experiment: deferred to clean-host review

The tested candidate is preserved in `7e07db7f6818c1bd2f29f828de72b6148a1dc89c`
on `codex/string-cache-dispatch-experiment-20261031`. It is **not part of the
accepted runtime**. Accepted runtime source remains `fd7b764`; the experiment
passes correctness and allocation gates, but local QuickSort timings remain
inconclusive. Further laptop repetition is not an acceptance strategy.

## Implementation and verification

The sparse immutable-code cache retains the first accessed string inline, adding
spill storage only for other accessed strings. Code edits invalidate caches;
returned strings retain copy-on-write semantics. Unlike the rejected sparse
wrapper, both dispatch paths check constant kind inline and call the outlined
cache helper only for strings. Generated code in the symbolized allocation probe
confirms direct non-string conversion in main and JIT-callback dispatch. This is
not direct disassembly of the stripped CLI or proof of total runtime speed.

- 1,312 library tests and 17 targeted integrations pass; eight existing library
  tests remain ignored. Exact suite/test membership and source hashes verified.
- All 150 test-size benchmark checksums pass across VM, interpreter and JIT.
- All 72 comparison checksums pass: 48 primary observations, 12 initial MergeSort
  screen observations and 12 bounded QuickSort follow-up observations.
- 24 semantic control runs and 186 allocation observations cover hot literals,
  cold unused constants, append snapshots and 1/4/16-key spill cases. Hot/cold/
  warm/append gates and exact warmed spill-allocation slopes pass.
- Host edits, shared code, concurrent cache users, Unicode, imported recursion,
  callbacks and independent returned values have regression coverage.

The candidate removes two requests and `key_width + 40` requested bytes per
measured VM literal read. Unused pools of 0/128/2,048 constants retain the same
cold allocation count. JIT's matched literal penalty was already zero.

Memory tradeoffs remain explicit. Empty code owners grow from 432 to 472 requested
and retained allocator bytes (+40); all tracked bytes are released after owner
drop. A cold 16-key/8-byte first-two-read phase grows by 335 bytes. Its warmed
2,000-read phase saves 1,536,061 bytes, including a fixed 61-byte marker saving;
1,536,000 bytes are attributable to repeated literal reads. Cached payloads live
with their immutable code owner, and spill-map synchronization remains. These
measurements are neither RSS nor a whole-program retained-heap guarantee.

## Timing evidence and disposition

All paired observations are retained. Percentages below are median candidate
changes versus the frozen accepted baseline; negative means faster.

| Workload | Runtime | Pairs | Warm elapsed | Process-tree CPU |
| --- | --- | ---: | ---: | ---: |
| QuickSort, primary | VM | 4 | +8.4% | +9.3% |
| QuickSort, follow-up | VM | 6 | -1.8% | -0.8% |
| QuickSort, combined | VM | 10 | +1.5% | +1.2% |
| QuickSort | JIT | 4 | -0.9% | -2.6% |
| MergeSort, combined screen and primary | VM | 10 | -2.9% | -0.3% |
| MergeSort | JIT | 4 | -5.5% | -2.7% |
| NeuralNet | VM | 4 | -7.3% | -8.2% |
| NeuralNet | JIT | 4 | +0.6% | +1.4% |

NeuralNet VM improves in all four elapsed and CPU pairs. QuickSort VM is slower
in five of ten elapsed pairs and six CPU pairs; elapsed ratios range from 0.933
to 1.389. Its six follow-ups contain four faster elapsed pairs, so the initial
slowdown is not consistent, but the aggregate does not prove neutrality. MergeVM
is slower in four of ten elapsed pairs and five CPU pairs; the previous wrapper's
consistent slowdown does not recur. NeuralNet JIT remains mixed.

These are loaded-laptop test-size observations. Process CPU includes the runner,
child startup, compilation, warmup and workload; it is not kernel-only CPU time.
Thermal/load/cache changes limit causality. No clean-host, production-size,
competitor, universal no-regression or overall-language speedup claim follows.

**Decision:** preserve the candidate for a dedicated-host comparison, without
merging it into the accepted runtime. Allocation benefit is established; the
QuickSort performance tradeoff remains unresolved. Do not discard the adverse
observations or repeatedly retest until the medians turn favorable.

## Reproducible evidence

`string-cache-dispatch-experiment-2026-10-31.json.gz` contains digest-indexed raw
reports, source manifests, test output, comparison/verification scripts, generated
code, footprint evidence and the runtime patch. The tested source manifest was
rechecked against the experiment commit before restoring the accepted branch.
Baseline allocation reports remain in the prior committed string-constant and
sparse-cache bundles. The CPU-accounting control is explicitly reused from the
prior verified same-host control, with provenance; it was not rerun here.

Frozen candidate CLI SHA-256:
`3279b9972ce4720a4fef8009405edcea328ea80d142bd4f60277e5d1eb20af78`.
Candidate library:
`0b03ab66f129a749c179d7726a89836a110d348cacbef396a62e5db6d94d6e83`.
Accepted baseline CLI:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

`target/release` contains experimental artifacts and is not the accepted baseline.
No release, upstream LangArena submission or Kostya contact occurred.
