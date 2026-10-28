# Nested dictionary leaf publication — 2026-10-28

Runtime `2caf05f` avoids repeated full-leaf dictionary copies when a captured
helper updates one existing scalar key under a dictionary, struct or array owner.
Baseline runtime: `924e949`.
[Machine evidence](nested-dictionary-publication-2026-10-28.json) preserves source,
artifact and fixture identities, measurements, verification code and checksums.

## Measured results

Requested allocation per added helper call at leaf width 2,048, initial-only snapshot:

| Owner / key | Baseline bytes | Candidate bytes | Baseline requests | Candidate requests |
| --- | ---: | ---: | ---: | ---: |
| Dictionary / numeric | 632,275 | 4,804 | 53 | 47 |
| Dictionary / string | 632,410 | 4,894 | 59 | 51 |
| Struct / numeric | 632,280 | 4,801 | 57 | 52 |
| Struct / string | 632,415 | 4,891 | 63 | 56 |
| Array / numeric | 631,646 | 4,718 | 47 | 43 |
| Array / string | 631,781 | 4,808 | 53 | 47 |

Eligible costs are flat across widths 2/128/2,048. The target has 144 paired
allocation measurements and 432 combined baseline/candidate semantic runs,
covering all three owners, both key forms, initial/fresh snapshots and 16/32 calls.
There are no measured total or added-call allocation regressions. The unchanged
baseline correctly fails the maintained improvement policy's width check.

These are requested allocation measurements, not elapsed speed or retained heap.
The loaded laptop cannot establish competitive performance against Goose/Rust.

## Semantics and limits

The private assignment path retains the canonical existing key and scalar value
without enlarging ordinary assignment or capture storage. Admission requires the
captured root to be shared only with its owner and the leaf to be unique. Repeated
same-key updates coalesce; other values, keys and paths retain fallback.

Owner-aware publication preserves real snapshots. Numeric/string aliases, exact
scalar values, callback observation, completed writes before errors, and root
replacement retain their behavior. Reads preserve object-before-index evaluation;
assignments update the replacement root as specified.

A separate 240-pair fallback matrix and 360 semantic cases pass: whole-root/leaf
escapes, shared leaves, snapshots followed by writes, second keys, insertion,
callbacks, nonscalar values and invalid operands. An additional 36 cases match
baseline stdout, stderr and exit status for root-replacing reads/assignments.
Expected JIT-mode import fallback notices remain visible; these checks do not
claim that imported helpers execute entirely as native JIT code.

## Verification

- 1,205 library tests pass; eight existing tests are ignored.
- 20 nested component tests pass, including unchanged storage layout.
- Three new integration tests pass in VM, interpreter and JIT modes, covering
  36 runtime/owner/key cases; the same tests also pass on the accepted baseline.
- 382 matching-release integration tests pass; one manual profiling test is ignored.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- 48 diagnostic pairs and historical dictionary, array, row, ownership, capture,
  lookup, reentry, cold-call, append and publication controls pass.
- The separate 240-pair collection refresh passes without allocation regressions.
- Independent assembly verifies exact coverage, fixture/artifact hashes, computed
  deltas, negative policy control and the recorded runtime/checksum gates.
- Formatting, diff and frozen-source checks pass.

## Remaining work

Sections 3 and 4 remain open for deeper paths, multiple nested leaf keys, larger
rows, general capture traversal/materialization, retained-name/prefix costs and
oversized fallback. Required snapshot copies remain. Production-size completion,
platform/release acceptance and clean-host speed comparisons are still pending.
