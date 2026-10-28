# Captured dictionary scalar publication

Runtime commit `36b8006d1bd3af2883a6f22e6d839efb3c42ea11` passes the full local correctness and allocation gate.
See [machine evidence](dictionary-scalars-2026-10-27.json) for inputs and identities.
Baseline runtime: `3d06ea2`; candidate binary SHA-256
`5754223421041e2a61746ac25cc3350cc5a8e77af3f33c1dcb5dac04e7a733bf`.

The candidate extends the existing single-key captured dictionary journal from
integers to floats, Booleans and null. Typed payloads carry their kind in the
existing capture-journal enum. Type changes move the stored name, dictionary and
key without allocating another journal or cloning the dictionary. Actual journal
and enclosing Option layout tests retain the prior size and alignment.

Read, snapshot, publication, rollback and journal-selection paths handle all four
types. Different-key and unsupported-value writes retain the existing fallback.
Public owners remain unchanged before publication, and retained snapshots remain
independent. Rust tests check exact float bits, including negative zero and a NaN
payload, scalar type identity, owner replacement and atomic rejection.

## Allocation results

The scalar target covers five value patterns (integer, float, Boolean, null and
mixed), widths 2/128/2,048, initial-only/fresh-per-call snapshots and 16/32 calls.
All 120 paired allocation and 180 semantic cases pass. Noninteger initial-snapshot
costs improve, widths 128/2,048 have equal added-call costs, and VM/JIT deltas agree.
The integer control stays unchanged. No total or added-call request/byte regression
occurs in the matrix. Fresh snapshots retain their required copying.

Per added call at 2,048 keys, initial snapshot retained:

| Value pattern | Baseline bytes | Candidate bytes | Baseline requests | Candidate requests |
| --- | ---: | ---: | ---: | ---: |
| int | 4806.0 | 4806.0 | 59.0 | 59.0 |
| float | 631601.0 | 4806.0 | 63.0 | 59.0 |
| bool | 631601.0 | 4806.0 | 63.0 | 59.0 |
| null | 631605.0 | 4810.0 | 64.0 | 60.0 |
| mixed | 474903.25 | 4807.0 | 62.25 | 59.25 |

A separate matrix covers returning the root dictionary, returning it inside an
array, and taking a snapshot before another write. Across four scalar types,
widths 2/2,048, both snapshot modes and 16/32 calls, all 192 allocation pairs and
288 semantic cases pass without total or added-call allocation regressions.

Accepted row-update behavior also passes 288 allocation pairs and 432 semantic
cases in explicit control mode. Its default mode still requires an improvement;
control mode retains all nonregression and flat-width checks. Synthetic policy
checks reject extra bytes and lost width scaling. The invalid-root operand matrix
passes another 96 allocation pairs and 144 semantic cases.

The independent target verifier checks exact Cartesian coverage, source and
fixture identities, recomputed deltas and matching artifacts. The unchanged
baseline correctly fails the new scalar improvement gate after the full matrix.
These are requested-allocation measurements, not elapsed-speed or retained-memory
claims. Laptop load is not used to accept or reject speed improvements.

## Verification

- 1,190 library tests pass; eight existing tests remain ignored.
- Nine targeted integration tests pass, including 192 scalar-transition scenarios
  across VM, interpreter and JIT, with snapshots, callbacks and errors.
- 358 matching-release integration tests pass; one manual profile remains ignored.
- All 18 baseline/candidate nested-assignment parity cases pass.
- The target/control matrices above pass 696 allocation pairs and 1,044 semantic cases.
- Historical split-owner, foreign-callback, direct-call and local-capture controls
  pass: 144, 144, 96 and 48 allocation pairs, respectively.
- Dictionary reads, numeric keys, module state and nested collection controls pass:
  16, 12, 128 and 72 pairs, respectively.
- All 36 cold comparisons, eight append pairs and three publication-boundary controls pass.
- All 150 unchanged test-size checksums pass: 50 each in VM, interpreter and JIT.
- Formatting, diff, frozen source and benchmark identities pass.

The independent assembler verifies matching artifacts, exact target coverage,
fixture hashes, recomputed deltas and unchanged-baseline rejection. The committed
escape runner is byte-identical to the runner used for its recorded measurements.
No benchmark algorithms, inputs or checksum expectations changed.

## Remaining scope

Multiple dictionary keys, nonscalar values, deeper paths, larger/dense/escaping
collection costs, broader capture materialization and compiled-execution work
remain open. Full production, platform and clean-host Goose/Rust acceptance are
not implied. No Kostya contact occurred.
