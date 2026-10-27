# Compact captured-local lookup storage — 2026-10-27

Runtime `82bdef6` reduces the storage used by oversized private capture lookups
without changing public owner values, snapshots, pool limits or Environment fields.
Small captures keep their existing representation. Private local scopes with at
least 8,192 names cannot fit the existing pool bound; they store complete name/kind
membership in the existing kind maps instead
of allocating Value-sized Null placeholders. Dense writes publish to the owner
without rebuilding those placeholder tables.

Baseline: accepted runtime `4f5087b`. [Machine evidence](compact-capture-views-2026-10-27.json)
records matching release artifacts, frozen sources, paired measurements, checksum
outputs, verifier sources and the rejected first candidate.

## Measured allocation reduction

Requested traffic per additional outer invocation, depth-zero writes:

| Retained locals | Requests, before and after | Before bytes | After bytes |
|---:|---:|---:|---:|
| 0 / 128 / 2,048 / 4,096 | 41 | 1,654 | 1,654 |
| 8,192 | 8,240 | 2,721,367 | 624,215 |
| 12,288 | 12,336 | 2,764,615 | 667,463 |
| 16,384 | 16,432 | 5,447,495 | 1,253,191 |

VM and JIT deltas agree across every target group. The reduction is about 76–77%
for the oversized rows; retained-name cloning still accounts for request growth.
These are allocation bytes, not elapsed speed or peak memory.

The two matrices contain 288 paired observations and 432 semantic executions.
The repeated 8,192-local width leaves 252 unique allocation cases and 378 unique
semantic cases. All 12 oversized depth-zero read/write groups strictly reduce
bytes without increasing requests. Total and added-call allocations do not regress.
Extra recursive overhead does not increase and remains flat within each unchanged
pool regime. An unchanged-baseline synthetic control is rejected by the target gate.

Dense all-key writes also pass: 12 paired cases and 18 semantic runs across
128 / 8,192 / 16,384 locals and one/four calls. At 16,384 locals, bytes per added
call fall from 5,503,971 to 1,309,667 with unchanged requests. Total and added-call
allocation controls pass, including independent captures and aliases.

## Regression found and corrected before acceptance

The first candidate passed its initial library and release tests but failed an
additional Rust embedding reproducer. Removing an immutable local from a public
owner before invocation left stale kind metadata. The candidate incorrectly
retained local membership and hid a global fallback: baseline returned 11 while
the candidate reported an undefined variable. That candidate was rejected and
its partial evidence was preserved; it was never committed.

Capture admission now filters local kind metadata against actual source names.
Removal after admission still retains the invocation's original lexical membership.
Unit tests cover both orders at 0 / 128 / 8,192 locals. The permanent host embedding
regression passes in debug and the matching release integration harness.

## Verification

- 1,184 library tests passed; eight ignored. All 29 targeted integration tests passed.
- 355 matching release integrations passed; one manual profiling test ignored.
- All 50 test-size benchmark checksums passed separately in VM, interpreter and JIT:
  **150 total**. This is not production-size certification.
- Six supplemental two-helper-hop release contracts and 18 nested-assignment
  baseline/candidate parity cases passed.
- Read, numeric-key, module, nested, direct, foreign, ordinary-local, cold, append
  and publication-boundary controls passed without tested allocation regressions.
- Dense mutation, immutable guards, duplicate declarations, host edits/expiration,
  detached/generator snapshots, callback lock release and rejected mutation journals
  have regression coverage.
- Formatting, diff checks, 825 frozen source hashes, benchmark-source hashes and
  matching executable/library/probe identities passed the evidence validator.

Candidate executable SHA-256:
`f66b6d70afe4d32b6c50443d49a53b7c526860b326073d11a01ae730ffb2e1e5`.
Candidate library SHA-256:
`63e3d53f6c82011449e47c227764f33ceba9d1b9e0310eb2653a7cbe64d61dc4`.

The [allocation gate](probes/compact-capture-allocations-gate.rb) accepts the
original-width report, extended-width report and a new output path. Exact measured
runner sources, fixtures, compiler arguments and hashes are retained in the machine
evidence. The audit root is `.audit-evidence/capture-key-view-v2-2026-10-27`;
`pipeline.rb` and `supplemental.rb` record the complete verification stages.

## Remaining scope

General capture traversal and retained-name cloning, broader nested/multiple-key
collection detachment and compiled execution remain open. Full production runs,
clean-host Goose/Rust comparisons and platform/release acceptance remain pending.
This milestone does not complete the full performance checklist or establish a
competitive speedup. No Kostya contact or upstream submission occurred.
