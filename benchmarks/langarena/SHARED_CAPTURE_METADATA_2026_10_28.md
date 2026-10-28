# Shared captured metadata — verified allocation reduction

Runtime commit: `6d288612375548ae9ca941776d20d37df51c0420`. Baseline: `4d1195238008792349a8054815c354f0c075d896`.
[Full evidence](shared-capture-metadata-2026-10-28.json) includes frozen source,
binary/library/probe identities, actual build/link arguments, fixtures, reports,
verifiers and rejection evidence. The local toolchain is Rust 1.96.0.

Oversized captured-local owners reuse compact name/kind metadata after exact
membership validation. Names, entries and buckets use boxed storage; active views
share one metadata allocation with copy-on-write binding kinds. Ordinary small
owners keep their existing representation. Value snapshots, public owner values,
immutable-binding rules and pool limits remain intact. Membership validation still
traverses names; this change does not eliminate all capture setup work.

## Allocation result

Per added call at probe width 8,192 (extra retained declarations, plus helper state); VM and JIT agree:

| Operation | Baseline requests | Candidate requests | Baseline bytes | Candidate bytes |
| --- | ---: | ---: | ---: | ---: |
| Read | 31 | 27 | 311,959 | 1,650 |
| Write | 32 | 28 | 311,970 | 1,661 |

All 48 target allocation pairs and 72 semantic cases pass. Total and per-call
requests/bytes do not grow in this matrix. Split reentry (144 pairs/216 semantics),
cold calls (36 pairs/54 semantics), and dense updates (12 pairs/18 semantics) pass.
The cold report contains 72 raw observations, not 72 pairs. An independent verifier
checks exact matrices, source/artifact identities and recomputed deltas.
These are requested-allocation measurements, not elapsed-speed improvements.

## Correctness and rejected drafts

- 1,229 library tests pass; eight ignored profiling/optional tests remain ignored.
- All three targeted capture tests pass in VM, interpreter and JIT. They are also
  included in the 403 passing integration tests; they are not 406 distinct tests.
  One manual integration profiling test remains ignored.
- All 150 test-size checksums pass: 50 each in VM, interpreter and JIT.
- Formatting, whitespace and frozen-source checks pass. Independent verification
  matches the exact 50 configurations and their checksum output in every runtime.

An initial draft required a full owner kind map and missed ordinary mutable owners,
whose kinds are implicit. An admission assertion reproduced that omission before
acceptance. A later candidate passed broad tests but a new host-scope-shrink test
exposed stale mutable-name retention. The final implementation discards stale
membership at the next capture admission while retaining immutable exceptions for
possible host reinsertion. Existing views retain independent metadata. The new
regression fails on the prior candidate and passes on this runtime; its two final
cases cover a small replacement scope and an oversized replacement scope.
All 48 target allocation observations remain exactly equal after this correction.

The integration harness runs the three targeted tests by filter before the full
corpus, avoiding a duplicate test-executable compilation without dropping coverage.

## Memory tradeoff

This cache intentionally retains metadata between calls. In the isolated component,
the exact 8,192 generated names retain **244,746 bytes** per owner, versus zero
retained metadata in the old transient design. The exact 16,384-name component
retains 496,890 bytes per owner. Retention scales
with owner count, not call count; metadata allocations are freed when their last
owner/view drops. This is component accounting, not a claim that RSS immediately
falls when objects are freed. Stale mutable membership is evicted at the next
capture admission after a host scope change; removed immutable exceptions survive
for semantic correctness.

Whole-process peak RSS was measured separately using two alternating pairs per
group. Each fixture validates independent owner state and keeps owners alive.
Runtime widths count extra declarations; the helper also captures its counter,
so these are not the exact same name sets as the isolated component:

| Extra declarations | Owners | Calls per owner | VM mean peak RSS change | JIT mean peak RSS change |
| ---: | ---: | ---: | ---: | ---: |
| 128 | 1 | 1 | +0.60% | +0.00% |
| 128 | 1 | 16 | +0.03% | +0.00% |
| 128 | 8 | 1 | +0.26% | +0.05% |
| 128 | 8 | 16 | +2.01% | -1.02% |
| 128 | 32 | 1 | +0.28% | -1.20% |
| 128 | 32 | 16 | +0.00% | +0.84% |
| 8192 | 1 | 1 | +7.79% | +7.17% |
| 8192 | 1 | 16 | -1.35% | +3.63% |
| 8192 | 8 | 1 | +2.96% | +0.98% |
| 8192 | 8 | 16 | +5.87% | +2.63% |
| 8192 | 32 | 1 | +3.62% | +2.90% |
| 8192 | 32 | 16 | +3.23% | +3.66% |

The 32-owner/width-8,192 groups add about 7.7–9.7 MB of mean peak RSS (2.9–3.7%).
Smaller groups show wider relative variation; all raw pairs are retained in the
report. Two local pairs are descriptive evidence, not a clean-host confidence
interval or a universal memory bound. The cache retention is an accepted, explicit
tradeoff for removing repeated large metadata allocations. This is not a
memory-neutral optimization, and no elapsed-speed claim follows.

## Remaining work

Sections 3 and 4 remain open for capture traversal/materialization beyond this
eligible path, remaining module collection copying, and representation review.
Insertion, nonscalar replacement, dynamic/deeper loops, dense updates and escaping
state require attribution against required snapshots. The current follow-up
isolation is separate from this fix. Production-size, other-platform and clean-host
performance acceptance remain separate; the test-size checksums do not close them.
