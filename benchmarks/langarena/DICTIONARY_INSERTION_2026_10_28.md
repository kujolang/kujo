# Bounded root dictionary insertion — accepted local checkpoint

Runtime `4cd3bf1f2b998a80db15c749571e30f60d759564` avoids whole-map copies for eligible new scalar entries in
captured root dictionaries. The first key stays inline. Proven batches use sparse
storage only within a conservative allocation budget; small or uncertain cases
retain ordinary mutation. Non-batched captured-counter helpers also benefit.
Snapshots, callbacks, errors, canonical key aliases and root replacement retain
existing semantics. New proof and fixture tests cover bounds, fallback, promotion
and independently observable old values.

## Measured result

| Original counter helper | Baseline bytes / added call | Candidate | Requests |
| --- | ---: | ---: | ---: |
| Width 128, VM and JIT | 47,320 | 8,069 | 89 → 86 |
| Width 2,048, VM and JIT | 634,840 | 8,069 | 89 → 86 |

The latter is a 98.7% reduction in requested allocation bytes for this control.
It is not an overall speedup, retained-memory bound or Goose/Rust comparison.

## Verification

- 1,236 library tests passed; eight ignored.
- 404 integration tests passed; one manual profiling test ignored.
- All 50 canonical test-size checksums passed in VM, interpreter and JIT (150).
- 2,172 paired allocation controls: 160 cold, 80 warm/cold root, 480 dynamic,
  400 factor/pair/wide/nested and 1,052 supplemental. Each matrix passed its
  total/slope checks; these are observations, not unique language features.
- 2,970 corresponding semantic cases, plus the insertion fixture in three modes.
- Exact source, binary, library, fixture and complete test-set verification passed;
  formatting and diff checks passed.
- 36 memory pairs / 72 executions across three modes completed after builds.

Whole-process peak RSS fell about 25.5–26.1% for the large unaliased-owner controls.
Empty/retained controls had mean changes between -0.04% and +1.30%; retained
width-32,768 VM was +880,640 bytes. Both rounds and every positive difference are
preserved in the JSON. Two local observations per case cannot establish a zero
memory regression or distinguish retained heap from allocator/OS effects. The
allocation controls pass; required snapshot copies remain. Acceptance is scoped
to correctness and measured allocation behavior, not competitive speed.

V1 remains rejected for cold byte regressions; V2 remains incomplete because it
missed the counter helper. Verification-tool errors, including an invalid receipt
from the first checksum-cache verification attempt, are quarantined in the local
audit. The corrected independent verifiers passed; no runtime failure was hidden.

## Remaining sections 3 and 4

1. Nested initial insertion. The root fix does not remove the measured nested cost.
2. Collection-valued replacement while retaining old arrays/dictionaries.
3. Broader compound-index and captured-scalar batching.
4. Capture traversal/materialization and oversized fallback: fix avoidable work,
   document necessary work.
5. Collection representation review and final combined-candidate verification.

This closes the current root-insertion review cutoff, not sections 3 and 4.
Allow several more implementation/verification cycles, likely days rather than
hours. This is a planning estimate, not measured remaining duration. There is no
credible remaining-time percentage or comparison with time already spent.
Clean-host speed comparisons remain pending separately; section 5 is not restarted.

Evidence: [machine-readable report](dictionary-insertion-2026-10-28.json).
Raw artifacts and verifier sources: `.audit-evidence/dictionary-insertion-v3-2026-10-28`.
