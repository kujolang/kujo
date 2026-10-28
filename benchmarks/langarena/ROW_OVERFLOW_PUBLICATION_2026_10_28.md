# Larger row publication — 2026-10-28

Runtime `c83ee4b9d1dd7b12563ee0c7c6a9bd550ec42534` extends private scalar row publication beyond four
indices for admitted explicit assignments and bounded counted loops. The accepted
comparison runtime is `ca3403d`; the [baseline](ROW_OVERFLOW_ISOLATION_2026_10_28.md)
reproduces the original copy cost. No benchmark algorithm was changed.

## Measured allocation results

Requested bytes per added call, dictionary owner, row width 2,048 and initial-only
snapshot. Each helper also updates index zero using its negative-index alias.

| Distinct indices | Body | Baseline | Candidate |
| ---: | --- | ---: | ---: |
| 4 | loop | 6,326 | 6,326 |
| 4 | explicit | 6,320 | 6,320 |
| 5 | loop | 286,036 | 6,922 |
| 5 | explicit | 286,029 | 6,915 |
| 16 | loop | 290,689 | 11,702 |
| 16 | explicit | 290,671 | 11,684 |

Candidate added-call allocation is flat across widths 128/2,048 in these initial-only
snapshot cases, with matching VM/JIT deltas. All 288 target allocation pairs and
432 semantic cases pass. Total and added-call requests/bytes must not regress.
Fresh snapshots retain required copies. These are allocation traffic measurements,
not retained memory or elapsed-speed comparisons against Goose/Rust.

## Implementation and rejected candidate

The existing four-entry inline journal is unchanged. An admitted fifth distinct
index creates private overflow storage keyed by normalized indices. Negative aliases
coalesce; snapshots and publication preserve copy-on-write isolation. Assignment
and activation layout remain unchanged.

Admission checks a single shallow row owner, scalar expressions and index bounds.
Counted loops use one integer local, an immutable literal/argument bound and a
positive constant increment as their final statement. The body is checked once;
monotone index endpoints establish bounds, and scalar range checks are cached by
positive/negative counter polarity. Admission allows at most 65,535 iterations.
Uncertain bodies, escapes, shadowed inputs and unsupported paths retain fallback.

The first candidate passed all 150 test-size checksums and allocation gates but was
rejected for repeated loop-body preflight on an early-error path. With 100 repeated
statements and 10,000 iterations, it inspected one million assignments per helper
call even though execution failed at the first division. Recorded VM user CPU was
1.09–1.12 seconds versus 0.016–0.018 for the baseline. Rejection evidence and the
original source patch are retained in the evidence JSON.

The revised guard's deterministic unit test requires 100 assignment inspections,
zero row reads for write-only assignments, and one 10,000-element scan for repeated
read/update statements. It also covers strides, negative indices and interior
nonscalar values. The same early-error diagnostic was rerun with two observations
per build and runtime, ordered baseline/candidate/candidate/baseline:

| Runtime | Baseline mean user CPU seconds | Candidate mean user CPU seconds |
| --- | ---: | ---: |
| vm | 0.019994 | 0.018708 |
| interpreter | 0.022716 | 0.020072 |
| jit | 0.018814 | 0.017648 |

This is a coarse local regression check, not clean-host percentage acceptance or a
claim about general throughput. The machine's background load varies. JIT-enabled
execution reports VM fallback for the import-containing script; these observations
do not establish generated-native coverage.

## Verification

- 1,222 library tests pass; eight existing tests ignored.
- 397 matching-release integration tests pass; one manual profiling test ignored.
- All 150 unchanged test-size benchmark checksums pass across VM, interpreter and JIT.
- 288 target allocation pairs / 432 semantic cases, independently verified.
- 832 larger escape pairs / 1,248 semantic cases, independently verified.
- 512 deep fallback pairs / 768 semantic cases, independently verified.
- Full-row regressions cover unique owners, fresh snapshots, aliases, callbacks,
  negative indices, replacement roots and partial writes before errors.
- Prior collection/capture/cold/append controls, source identities and formatting pass.

Raw total allocation comparisons use baseline reruns at the candidate's exact
fixture paths. An initial comparison against historical different-path totals was
rejected: import/setup bytes varied with path length while added-call deltas matched.
All 288 same-path calibrations and historical per-call checks are retained. The
synthetic unchanged baseline correctly fails the required-overflow-improvement gate.

The escape runner's legacy stdout count is incorrect; its JSON and independent
coverage verifier establish the 832/1,248 counts above. Dedicated older dictionary
spill/deep-publication matrices remain historical; they are not represented as
current reruns. The current gates include deep fallback and broad collection controls.

## Remaining scope

General/deeper loops, nonscalar updates and uncertain control flow retain fallback.
Sections 3/4 remain open for capture traversal, retained names/prefixes, oversized
owners and broader collection/representation costs. Required snapshots are not
performance defects. Full production, platform/release and clean-host comparative
speed acceptance remain separate gates.

Machine-readable evidence: [row-overflow-publication-2026-10-28.json](row-overflow-publication-2026-10-28.json).
