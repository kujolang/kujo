# Bounded nested-row publication — 2026-10-27

Runtime commit `3d06ea242d798679d9e2f219023c123f791e386a` retains up to four distinct scalar writes to one
captured struct/dictionary array row in the existing assignment-path storage.
Repeated writes to the same resolved index coalesce. A fifth distinct index uses
the existing materialization path. Public owners remain unchanged before
publication, and retained snapshots preserve ordinary copy-on-write semantics.

Baseline runtime: `82bdef6`. The [machine evidence](nested-row-batch-2026-10-27.json)
contains matching source, executable, library, probe and fixture identities,
compiler arguments, raw results and verification scripts. These are local
allocation and correctness results, not elapsed-speed measurements.

## Allocation results

Helpers update 1/2/3/4/5/16 distinct elements and repeat the first through a
negative index. The matrix covers struct/dictionary roots, widths 32/128/2,048,
initial-only/fresh-per-call snapshots and 16/32 calls. All 288 allocation pairs
and 432 semantic cases pass. VM/JIT call deltas agree. Two-to-four-entry costs
stay flat from 128 to 2,048 elements, with no total or added-call request/byte
regressions in the matrix.

Requested bytes per added helper call at 2,048 elements, initial snapshot retained:

| Root | Distinct elements | Baseline | Candidate |
| --- | ---: | ---: | ---: |
| struct | 1 | 284444 | 5144 |
| struct | 2 | 284784 | 5484 |
| struct | 3 | 285129 | 5824 |
| struct | 4 | 285474 | 6164 |
| struct | 5 | 285819 | 285804 |
| struct | 16 | 289614 | 289599 |
| dictionary | 1 | 284516 | 5186 |
| dictionary | 2 | 284896 | 5566 |
| dictionary | 3 | 285319 | 5946 |
| dictionary | 4 | 285742 | 6326 |
| dictionary | 5 | 286165 | 286036 |
| dictionary | 16 | 290818 | 290689 |

Five/sixteen-entry updates still scale with row width. The bounded optimization
does not close larger-batch, deeper-path or general dictionary-update costs.

## Semantics and rejected candidates

The actual assignment enum size/alignment and nested-publication storage remain
unchanged. Recording four slots adds no journal allocation. Rejection of a fifth
distinct index is atomic; repeated aliases remain eligible when all slots are full.

Checked negation of an integer literal or simple integer binding can read pending
row values without taking a speculative snapshot. Calls, unsupported expressions,
noninteger values and overflow retain ordinary evaluation and diagnostics.
Root-as-index guards avoid extra materialization for a leaf index, negated leaf
index or row selector. Completed writes still publish before subsequent errors.

V1 left negative-expression read copying and was stopped. V2 passed the row target
but failed expanded invalid-operand controls: speculative root lookups introduced
extra copies. V3 was stopped after the expanded V2 reproduction identified a
selector path unchanged in V3. These were deliberate corrections, not timeouts
or weakened acceptance rules. V4 passes the complete expanded matrix: 96 paired
allocation and 144 semantic cases across both roots, all three operand forms,
32/2,048 elements, both snapshot modes and 16/32 calls. No total or added-call
allocation regression remains in those controls.

## Verification

- 1,186 library tests pass; eight existing tests remain ignored.
- All 16 nested unit tests and three expanded VM/interpreter/JIT contracts pass.
- All 355 matching-release integration tests pass; one manual profile is ignored.
- All 18 baseline/candidate nested-assignment parity cases pass.
- Target matrices pass 384 allocation pairs and 576 semantic cases in total.
- Split-owner, foreign-callback, direct-call and local-capture allocation controls
  pass: respectively 144, 144, 96 and 48 paired cases.
- Dictionary read, numeric-key, module-state and broader nested controls pass:
  respectively 16, 12, 128 and 72 paired cases.
- All 36 cold comparisons, eight append pairs and three publication-boundary
  controls pass without allocation regressions where paired counts apply.
- Another 192 scalar-transition cases pass on both baseline and candidate with
  identical inputs, covering callbacks, snapshots and errors across all runtimes.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- Formatting, diff, frozen source, benchmark source and artifact identity gates pass.

The independent assembler verifies exact target matrices, fixture hashes,
recomputed deltas and artifact identities. The unchanged baseline fails the target
improvement gate as expected; the rejected V2 binary reproduces 32 regressions in
the 96-pair invalid-operand matrix. Raw evidence for both controls is retained.

## Remaining scope

Larger/dense or escaping updates, different rows, deeper roots, dictionary scalar
and multiple-key costs, general capture materialization and compiled execution
remain open in the [checklist](PERFORMANCE_CHECKLIST.md). This checkpoint does not
establish retained/peak memory, full production-size completion, platform acceptance
or clean-host Goose/Rust speed. Dedicated-host comparisons remain pending. No
Kostya contact occurred.
