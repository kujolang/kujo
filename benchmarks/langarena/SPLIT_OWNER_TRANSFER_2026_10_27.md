# Split captured-owner transfer — 2026-10-27

Runtime `4f5087b` removes repeated private capture-map copying when a shared
helper suspends captured locals separately from globals before foreign callback
reentry. Locals and globals return to their own suspended frames. Captured values
stay owner-backed; parameter scopes, snapshots, write journals and error unwinding
are preserved. Tasks keep their existing fallback. Pool limits and activation-frame
layout are unchanged.

Baseline runtime: `65a58f2`. [Machine evidence](split-owner-transfer-2026-10-27.json)
records sources, compiler arguments, artifact identities, paired measurements,
semantic results, negative gate control and all checksum outputs.
Candidate binary SHA-256:
`dd174045c51f304290b5db9ac0a8f31332fa748bd230d39c44c3a5efd7526267`.
Candidate library SHA-256:
`7dffaf8b6283048cd6197deb0f1a1211b57cb1f356b616312191c11755cd765c`.

## Measured improvement

Requested allocation per additional outer call, depth-four writes:

| Extra retained locals | Baseline requests / bytes | Candidate requests / bytes |
|---:|---:|---:|
| 0 | 560 / 35,443 | 503 / 30,742 |
| 128 | 944 / 160,285 | 503 / 30,742 |
| 2,048 | 6,704 / 2,069,989 | 503 / 30,742 |
| 8,192 | 41,540 / 13,630,073 | 8,712 / 2,752,085 |

VM and JIT allocation deltas agree. These are requested allocation traffic,
not elapsed-speed or peak-memory measurements. Oversized outer activation still
copies retained names; this fix removes the extra width-growing reentry cost.

After subtracting depth-zero activation, extra recursion cost is constant within
each unchanged pool regime:

| Retained locals | Read depth 1 requests / bytes | Read depth 4 | Write depth 1 | Write depth 4 |
|---|---:|---:|---:|---:|
| 0 / 128 / 2,048 / 4,096 | 112 / 6,785 | 458 / 29,060 | 113 / 6,792 | 462 / 29,088 |
| 8,192 / 12,288 / 16,384 | 124 / 8,799 | 468 / 30,690 | 125 / 8,806 | 472 / 30,718 |

The fixed boundary step remains. All 28 depth-four groups strictly reduce both
requests and bytes. Combined matrices contain 288 paired observations and 432
semantic runs; repeated 8,192-width cases leave 252 unique allocation cases and
378 unique semantic cases. No tested total or added-call allocation increases.

The unchanged-baseline synthetic control is rejected by the scaling gate. It tests
the verifier, not a new runtime comparison. The reused foreign gate emits an old
`original_exact_equality` description: machine evidence preserves it as inherited
metadata, explicitly not a split-candidate failure. Actual cross-regime equality is
computed separately and is false. The split candidate passed the predeclared
bounded-regime criterion; no gate was weakened after measurements.

## Verification and reproduction

- 1,179 library tests passed; eight ignored. All 21 targeted integration cases passed.
- 348 matching release integrations passed; one manual profiling test ignored.
- Six strengthened direct/foreign/shared-helper contracts cover global/local writes,
  parameter shadowing, immutable arrays, snapshots, aliases, independent captures,
  foreign state, caught errors and exact validation counts in all three runtimes.
- Six supplemental two-helper-hop contracts passed on debug and matching release.
- 18 nested-assignment baseline/candidate parity cases passed.
- Read, numeric-key, module, nested, direct, same-owner foreign, ordinary activation,
  cold, append and publication controls passed without allocation regressions.
- All 50 test-size benchmark checksums passed in VM, interpreter and JIT: 150 total.
- Formatting, diff checks and frozen-source identities passed.

Run `probes/split-local-allocations.rb` and
`probes/split-local-extended-allocations.rb` with matching candidate probe,
candidate binary, fresh output directory and baseline probe. Then run:

```sh
ruby benchmarks/langarena/probes/foreign-local-pool-boundary-gate.rb \
  ORIGINAL_REPORT_JSON EXTENDED_REPORT_JSON NEW_ACCEPTANCE_JSON
```

The original runner covers 0/128/2,048/8,192 locals; the extended runner covers
4,096/8,192/12,288/16,384. Exact fixture sources and identities are embedded in
machine evidence. Supplemental harness source is
`probes/split-owner-double-helper-contracts.rs`; compile with `rustc --test`, the
matching `tempfile` dependency and `CARGO_BIN_EXE_kujo` pointing at the candidate.
The regular six contracts remain in `tests/foreign_local_prefix.rs`.

## Remaining scope

General captured-environment materialization, oversized outer activation and broader
collection costs remain open. Later compiled-path, production-size, platform/release
and clean-host Goose/Rust acceptance remains on the [checklist](PERFORMANCE_CHECKLIST.md).
No competitive elapsed-speed claim or Kostya contact is implied by this result.
