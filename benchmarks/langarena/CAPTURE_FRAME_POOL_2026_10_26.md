# Bounded capture-frame storage reuse

Runtime commit: `6228389e18087d571943a27edac13dd5aed83c43`.
Baseline: `2c342ef1817bc6931feb84cbfc039f3dc1f26f1f`.
[Machine-readable evidence](capture-frame-pool-2026-10-26.json) retains source hashes, matching normal-release binary/library identities, test outcomes, checksums and raw allocation controls.

## Change and preserved behavior

Private invocation frames now reuse cleared maps, binding names and binding-kind tables. Every activation consults actual owner keys, values and mutability metadata. Recycling releases all runtime values, environment identities, captures, handles and native-default metadata. Only private map storage remains cached.

The pool is bounded to four frames, 8,192 total map capacity and 128 KiB of reserved key storage; frames with outer scope-vector capacity above 128 are rejected. Oversized owners fall back to dropping storage. These are storage bounds, not a measured peak-heap claim or a promise that every possible owner has flat setup costs.

Completed calls publish globals at the existing boundary, then clear retired storage outside the owner lock. Public owners retain real values and remain accessible during callbacks. Task snapshots, active reentry and the independent-helper proof retain their existing behavior. Recursive active-prefix snapshots are not pooled by this change.

## Measured allocation changes

These are normal-release **VM requested allocation traffic per added invocation**, calculated from the 64-to-128-call delta. They are not elapsed speed or retained heap.

| Helper/control | Baseline requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: |
| Parameter/local-only read, all tested binding/width controls | 21 / 1,241 | 21 / 1,241 |
| Module write, 0 extra bindings, 128 elements | 47 / 21,728 | 28 / 18,578 |
| Module write, 128 extra bindings, 128 elements | 175 / 61,410 | 28 / 18,578 |
| Module write, 2,048 extra bindings, 128 elements | 2,095 / 697,978 | 28 / 18,578 |
| Module write, 0 extra bindings, 2,048 elements | 47 / 282,848 | 28 / 279,698 |
| Captured read, 2,048 mutable extra bindings | 2,089 / 680,560 | 24 / 1,160 |
| Captured read, 2,048 immutable extra bindings | 4,138 / 835,114 | 24 / 1,160 |

Captured reads are flat at 24 requests / 1,160 bytes across the tested 0/128/2,048 extra bindings, mutable/immutable declarations and 128/2,048-element arrays. Their fixtures return nonzero captured values and verify accumulated results. They exercise capture setup, unlike the parameter/local-only read fast path.

JIT retains an additional **one request / 52 bytes over each 64-call delta** in these controls, both before and after. The gates compare slopes independently within each runtime and do not assume VM/JIT totals are identical.

Append controls at 512/1,024/2,048/4,096 elements remain linear. Each candidate VM/JIT run has a fixed **three fewer requests and 1,520 additional requested bytes** versus the baseline. This constant setup tradeoff is retained in the evidence.

## Verification

- **1,138 default-feature debug library tests passed; eight manual profiling tests ignored.** Five new unit tests cover released values/identities, public owner edits, mutability, stale metadata, storage bounds and publication equivalence.
- **289 matching normal-release integration tests passed; one manual profiling test ignored.** This includes the three public-owner callback contracts introduced in `394f675`. Ordered task/field screens also passed; they overlap this integration coverage rather than adding distinct tests.
- **150 test-size checksums passed:** all 50 unchanged LangArena fixtures in VM, interpreter and JIT. This is not a new full-production-size pass.
- **32 imported-helper allocation controls and 48 three-runtime semantic controls passed.** The strengthened [write-binding allocation gate](probes/imported-helper-allocations.rb) fails on the accepted baseline after its 32 runtime checks pass, and passes on the candidate. The original independent-read gate remains green.
- **32 captured-read allocation controls and 48 three-runtime semantic controls passed.** The new [captured-read allocation gate](probes/captured-helper-allocations.rb) rejects the baseline's setup growth and passes on the candidate, including immutable binding maps.
- **128 module-owner allocation controls and 192 three-runtime semantic controls passed** across arrays, dictionaries, struct arrays, dictionary arrays, one/eight updates and once/fresh snapshots. The fixed-width dictionary corpus matches the [ownership diagnosis](MODULE_OWNER_ISOLATION_2026_10_26.md).
- Formatting, whitespace, source identities and report consistency checked. The JSON lists ignored tests explicitly and preserves baseline gate failures as expected performance reproductions.

The allocation probes and runtime binaries must come from matching normal-release builds when rerunning the tracked gates. Both runners require a new output directory and preserve failed results.

## Remaining costs and limits

Collection-width copying is unchanged. Increasing width from 128 to 2,048 still adds 261,120 requested bytes per mutating invocation for arrays and nested-array shapes, and 587,520 for direct dictionaries. Fresh snapshots require separation; the once-snapshot cases and public-owner publication boundary still need a compatible ownership/representation solution.

Map traversal, value materialization outside the covered paths, recursive active-prefix snapshots and oversized-owner fallback remain relevant costs. Allocation reduction does not establish constant CPU work or a global language speedup.

The laptop is variably loaded. Clean-server speed acceptance and Goose/Rust comparisons remain pending, as do later compiled-execution work, unchanged full-production-size verification and platform/release gates. Sections 3 and 4 and the full performance goal remain open.
