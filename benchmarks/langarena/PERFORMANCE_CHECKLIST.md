# Kujo performance implementation checklist

Reviewed scope: 2026-10-26. This is the human-readable tracker for
`performance-plan.json`. Check a box only when its stated gate has evidence.
Historical passes do not certify a later runtime revision. A timeout is incomplete,
and a stopped case is cancelled, not failed. Sections 1 and 2 have completed their available local diagnosis. The user has
now authorized sections 3 and 4; section 5 has not been restarted.

## Completed foundation

- [x] Local port and checksum runner for all 50 benchmarks.
- [x] Assignment, nested collection, append/removal/pop and ownership fixes.
- [x] Reduced call/argument allocation and native-root bookkeeping.
- [x] Callback reuse, module-cache ownership and native code-identity fixes.
- [x] JIT arithmetic/type/error/assignment guards and broader native operations.
- [x] Worker JIT policy and cancellation propagation fixes.
- [x] Latest verified runtime `f415211`: 150/150 test-size checksums across VM,
  interpreter and JIT (`root-array-collection-publication-2026-10-29.json`).
- [x] Latest relevant gate: 1,254 library passes, eight ignored; 409 distinct
  integration passes, one manual profiling test ignored (`root-array-collection-publication-2026-10-29.json`).
- [x] One six-variant all-50 comparison (`nonleaf-comparison-all50-2026-10-25.json`).
- [x] Linux/Windows 21-runtime/3-static release corpus and pinned-source audit
  (`nonleaf-platform-partial-2026-10-25.json`). This is not LangArena production.
- [x] Partial production preserved: 13 JIT passes, five timeouts, GameOfLife
  cancelled; 31 JIT cases and both other modes not reached
  (`nonleaf-production-stopped-2026-10-25.json`).

## 1. Reliable measurement foundation

- [x] Reconcile the machine ledger with the latest committed evidence (diagnostic_review_2026_10_25).
- [x] Freeze reconstructed original, verified candidate, Goose, Rust, toolchains and inputs (`local-isolation-2026-10-25.json`).
- [x] Preserve original and current candidate as baselines for the next fix; repeat freezing after each future fix.
- [ ] Establish a quiet dedicated Linux host. **Blocked: user confirmed no host.**
- [x] Record CPU/architecture, OS, RAM, toolchains and build flags for the local candidate/probe (diagnosis manifest).
- [x] Record continuous load, memory/swap and available thermal telemetry during the main local pass.
- [x] Separate builds from measured benchmark intervals.
- [x] Run seven alternating local NeuralNet comparisons in VM and JIT; clean-host comparisons remain pending.
- [x] Record local phase checkpoints, allocation phases, compile counters, loop/total time and separate peak RSS observations.
- [x] Treat rounded zero competitor timings as below reporting resolution; do not calculate zero-based speedup ratios.
- [x] Keep NeuralNet comparison algorithms/configuration/checksums identical; label modified diagnostic inputs separately.
- [x] Preserve explicit timeout, cancellation, failed and passed statuses.

Gate: repeatable measurements with exact identities and uncertainty. A noisy local
diagnostic cannot close clean-host acceptance. Removed temporary executables have
now been reconstructed from pinned sources and hashed. The preserved candidate
matches its previously verified binary hash; no current release substitution.

## 2. Diagnose dominant costs

For every row below collect CPU samples, allocation requests/requested bytes,
copy/ownership evidence, call/frame/environment costs, native/fallback counters,
setup-versus-loop boundaries, a small reproducer, and a pre-change result.
Counters alone do not establish native CPU-time coverage. Allocation traffic is
not retained memory. A diagnostic input is not an official benchmark pass.

| Priority | Workload | Diagnosis gate |
| --- | --- | --- |
| First | QuickSort | [x] local profiles/phases and call/index/ownership controls |
| First | MergeSort | [x] local profiles/phases and call/index/ownership controls |
| First | NeuralNet | [x] seven-round local regression check, phases and struct-state controls |
| First | CacheSimulation | [x] depth/hash profiles and dictionary/nested ownership scaling |
| First | GameOfLife | [x] nested-state scaling, phases and validation attribution |
| Next | Brainfuck Array | [x] dispatch/operand profiles, phases and native counters |
| Next | Brainfuck Recursion | [x] dispatch/operand/call profiles, phases and native counters |
| Next | TextRaytracer | [x] field/root profiles, phases and struct-state controls |
| Next | Small JIT workloads | [x] separate compilation/setup costs and local iteration-count payback experiment |

- [x] Capture local CPU profiles with useful symbols; preserve short/missing-window limitations.
- [x] Measure aggregate main-thread allocation count/requested bytes separately from ordinary CLI peak RSS.
- [x] Isolate user snapshots, global loaded temporaries, function locals and assertion-call effects; retain required snapshots.
- [x] Measure warmed native call controls by arity/count, helper-return scaling, named-opcode clones and disabled-debug lookups.
- [x] Preserve native/fallback event evidence and sampled helper/dispatch categories without claiming CPU-time percentages.
- [x] Separate allocation phases and container-width/operation-count scaling; retain instrumentation boundaries.
- [x] Verify representative reproducers across VM, interpreter and JIT (495 ownership observations).
- [ ] Obtain precise generated-native/fallback CPU-time attribution. **Blocked locally:** xctrace absent; DTrace cannot initialize with current privileges/SIP.
- [x] Preserve local diagnostic before results before implementation.

Gate: each optimization proposal has a reproduced bottleneck, not just a static
hypothesis. Subsequent user authorization now covers sections 3 and 4.

Available local work in sections 1–2 is complete: [local isolation report](LOCAL_ISOLATION_2026_10_25.md).
Clean-host acceptance and precise native CPU-time attribution remain open.
The earlier [priority diagnosis](PRIORITY_DIAGNOSIS_2026_10_25.md) remains historical evidence.
The reproduced global indexed-write copy/validation cost was fixed in `787498d`. Local-state snapshot semantics
passed. The earlier NeuralNet VM slowdown did not reproduce consistently locally.

## 3. Allocation and call-frame fixes (verified fixes; remaining work explicit)

### Current round: remaining work and next review cutoff (2026-10-29)

Review status, 2026-10-30: sections 3 and 4 remain open. The remaining work is
grouped below; these groups are not equally sized and do not imply a completion
percentage or a reliable elapsed-time estimate.

1. Scalar-first array journal promotion is accepted in `ac7f234`.
   [Verified results](SCALAR_FIRST_BATCHES_2026_10_30.md) record 1,344 allocation
   comparisons, 2,664 semantic checks, three portable contracts, 1,266 library
   passes, 415 integration passes and all 150 test-size checksums. All 72 memory
   pairs are reviewed, including small increases. The next step is the remaining
   unsupported-shape investigation below.
   Initial string/byte batch routing is also accepted in `94ad8f5`:
   [verified results](STRING_FIRST_BATCHES_2026_10_30.md) record 1,728 allocation
   pairs, 3,240 semantic checks, 1,267 library passes, 416 integration passes,
   all 150 test-size checksums and 144 reviewed memory pairs. Mixed two-write
   updates are now accepted in `f92de4b`; bounded arithmetic selectors are also
   accepted in `5458132`.
   [Mixed-pair results](MIXED_PAIR_BATCHES_2026_10_30.md) record 2,496 allocation
   pairs, 4,392 semantic checks, 1,270 library passes, 418 integrations and all
   150 checksum cases. All 288 memory pairs were reviewed, retaining small
   increases (largest group mean +159,744 bytes / 2.09%). Eligible width-2,048
   mixed updates fall from roughly 284 KB to 6 KB per added call; this is an
   allocation result, not an elapsed-time speedup.
   The [expanded arithmetic-selector baseline](ARITHMETIC_SELECTOR_BASELINE_2026_10_30.md)
   verifies 192 allocation observations and 288 semantic checks on `94ad8f5`,
   plus a three-runtime overflow/bounds/callback contract. All seven arithmetic
   forms retain a 277,440-byte width penalty from 8 to 2,048 elements; the
   literal-index control is flat. The [verified fix](ARITHMETIC_SELECTOR_FIX_2026_10_30.md)
   now removes that width penalty for the seven measured addition/subtraction
   forms: 284,846→6,514 bytes/call (direct), 286,573→8,241 (loops) at width 2,048.
   Acceptance covers 2,688 allocation pairs, 4,032 semantic checks, 1,271 library
   passes, 419 integrations, all 150 checksums and 192 reviewed RSS pairs. Broader
   unsupported selectors and capture/representation review remain open.
   [Remaining selector isolation](REMAINING_SELECTOR_BASELINE_2026_10_30.md)
   confirms a 277,440-byte width penalty for invariant-parameter multiplication
   and division on the accepted runtime; 72 allocation observations and 108
   semantic checks pass. These measured shapes require a bounded follow-up fix.
2. Refresh remaining capture materialization and unsupported collection paths,
   including oversized fallback, general compound selectors and dense updates.
   [Dense root-array review](DENSE_ARRAY_REVIEW_2026_10_30.md) passes 48 allocation
   observations and 72 semantic checks: measured once-snapshot costs are linear
   in writes with no owner-width term; fresh snapshots retain separate copies.
   Dictionary/struct, deeper-loop and escaping shapes remain separate.
   Separate required snapshot/publication work from avoidable copying. Each
   confirmed avoidable cost needs its own bounded fix and regression gates;
   unmeasured shapes are investigation items, not confirmed defects.
3. Conclude the specialized representation review and verify the combined final
   candidate. Every remaining case must have an evidence-backed disposition:
   fixed, required by semantics, or explicitly deferred with its limitation.
   A deferred cost must not be described as fixed or as closing an unmet gate.

The scalar-first and initial string/byte patches are verified; the remaining investigation can produce
additional patches. Clean-host Goose/Rust comparisons and section 5 compiled
execution work are separate from this cutoff. Loaded-laptop elapsed times do not
establish competitive speed improvements.

Bounded root dictionary insertion is accepted in `4cd3bf1`; eligible nested initial
insertion is now accepted in `2c91c49`. [Nested results and limits](NESTED_DICTIONARY_INSERTION_2026_10_29.md)
record 1,239 library passes, 405 integrations, all 150 test-size checksums,
3,360 allocation pairs and 5,040 semantic checks. Both activation-layout tests pass.
The nested dictionary control falls from 634,405 to 6,977 requested bytes per added
call at width 2,048 in VM/JIT. This is not an elapsed-time speedup.

- [x] Nested initial scalar insertion, preserving snapshots, callbacks, errors and
  owners, with cold/fallback and conditional-batching regression coverage.

Work still needed, in priority order:

1. Collection-valued replacement without unnecessary owner-map detachment.
   [Accepted-runtime baseline](COLLECTION_REPLACEMENT_BASELINE_2026_10_29.md) confirms
   a 587,520-byte width penalty per added call for both array/dictionary values,
   with captured or argument counters; 64 allocation and 96 semantic checks pass.
   The [first replacement candidate](COLLECTION_REPLACEMENT_REJECTION_2026_10_29.md)
   is rejected: all target allocation totals and slopes remained unchanged.
   Literal evaluation cleared admission before assignment; the corrected candidate
   must repeat both target and fallback gates before acceptance.
   The [second replacement candidate](COLLECTION_REPLACEMENT_FALLBACK_REJECTION_2026_10_29.md)
   removes the target width penalty but is rejected for 48 two-key fallback
   allocation regressions. The compact-payload replacement is now accepted in
   `d853be4`: [verified results](COLLECTION_REPLACEMENT_2026_10_29.md) show flat
   root array/dictionary replacement costs, 1,244 library passes, 406 integrations,
   150 checksums, 3,760 allocation pairs and 5,640 semantic checks. All 54 memory
   pairs are preserved with their limitations. Struct-literal replacement costs
   remain unchanged; this does not close collection replacement as a whole.
   [Struct construction isolation](STRUCT_REPLACEMENT_BASELINE_2026_10_29.md)
   confirms that root argument payloads stay flat while in-helper literals add
   634,260 requested bytes per added call across widths 2/2,048. All 64 allocation
   observations and 96 semantic checks pass on the accepted runtime.
   [Nested replacement isolation](NESTED_COLLECTION_REPLACEMENT_BASELINE_2026_10_29.md)
   also confirms the same 587,520-byte width penalty under dictionary, struct and
   array roots: 96 allocation observations and 144 semantic checks. Root-only
   optimization does not close the nested portion of this item.
   [Deep/small-container baseline](NESTED_COLLECTION_DEEP_BASELINE_2026_10_29.md)
   adds 192 allocation observations and 288 semantic checks across deep dictionary,
   mixed struct/array paths and array leaves, including 1/2-call controls. VM/JIT
   slopes agree; fixed total-allocation offsets are retained separately.
   Eligible nested replacements are now accepted in `2a5ffbc`: [verified results](NESTED_COLLECTION_REPLACEMENT_2026_10_29.md)
   show flat target width slopes, 1,247 library passes, 407 integrations, all 150
   test-size checksums, 3,968 allocation pairs and 5,952 semantic checks. All 108
   memory pairs were reviewed, including small increases and local-measurement limits.
   Existing-key replacements are covered; struct-literal construction and unsupported
   collection paths remain open. [Struct boundary contracts](STRUCT_LITERAL_BOUNDARIES_2026_10_29.md)
   cover field ordering, callbacks, snapshots and errors for the next fix.
   The [first struct-literal candidate](STRUCT_LITERAL_REJECTION_2026_10_29.md)
   removes target width costs but is rejected for 128 fallback allocation totals
   and 64 slopes (+2 requests/+8 bytes per affected call). The ownership correction
   is accepted in `3ac637e`: [verified results](STRUCT_LITERAL_PUBLICATION_2026_10_29.md)
   record 1,249 library passes, 408 integrations, all 150 test-size checksums,
   864 allocation pairs / 1,296 semantic checks and 72 reviewed memory pairs.
   Struct field ordering, callbacks, errors and snapshots remain covered.
   [Direct root-array isolation](ROOT_ARRAY_REPLACEMENT_2026_10_29.md) confirms
   a remaining 286,440-byte width penalty per added call for struct replacements;
   32 allocation observations and 48 semantic checks pass. This unsupported direct
   array path remains open; the accepted correction does not optimize it.
   The [first direct-array candidate](ROOT_ARRAY_REJECTION_2026_10_29.md) is rejected:
   64 allocation totals / 32 slopes add 28 bytes per call on boxed struct fallback
   paths despite all 648 semantic checks passing. Profitability correction and
   full verification remain required.
   The V2 profitability correction passes all 648 targeted semantic checks and
   removes all 64 earlier allocation regressions, but remains unaccepted: one
   intermittent 10,320-byte allocation was traced to capture-map rehashing.
   A standalone deterministic map-churn control reproduces the growth mechanism;
   the exact runtime key transition and a safe correction remain under investigation.
   Broad candidate verification has not run past this failed allocation gate.
   The singleton capture-map correction is now accepted in `6ee5277`, and eligible
   direct-array replacements in `f415211`: [verified results](ROOT_ARRAY_COLLECTION_PUBLICATION_2026_10_29.md)
   record 1,254 library passes, 409 integrations, all 150 test-size checksums,
   1,232 allocation pairs / 1,848 semantic checks with no regressions, 15 additional
   scalar/compound contracts and 54 reviewed memory pairs. All 18 target groups
   have flat width scaling. Multiple-index updates and broader boxed-payload
   helper bodies still require work; this does not close collection replacement.
   Two same-kind array/dictionary payload replacements are now accepted in
   `e445f4a`: [paired replacement results](ARRAY_COLLECTION_PAIRS_2026_10_29.md)
   record 1,256 library passes, 410 integrations, all 150 test-size checksums,
   1,520 allocation comparisons / 2,280 semantic checks without regressions,
   15 additional scalar/compound contracts and 54 reviewed memory pairs.
   Four new hot target groups are flat across widths 3/2,048; all 18 previously
   flat groups remain flat. General multi-index writes, mixed payloads and broader
   boxed-payload helper bodies remain open; real snapshot costs remain separate.
   [General batch isolation](ARRAY_BATCH_BASELINE_2026_10_29.md) now confirms the
   remaining cost on `e445f4a`: 3/8/64-entry counted loops retain a 269,824-byte
   array/mixed or 277,760-byte struct owner-width penalty per added call across
   widths 64/2,048. All 288 allocation observations and 432 semantic checks pass;
   hot VM/JIT slopes agree. Three small cold JIT offsets are preserved separately.
   The [first general-batch candidate](ARRAY_BATCH_REJECTION_2026_10_29.md)
   removes the hot width penalty in all 18 target groups but is rejected for
   one first-use allocation in array-only cases. All 720 paired allocation
   observations and 1,728 semantic checks are retained; gates reject 72 totals
   and 24 cold slopes. Isolated default hash initialization reproduces one
   request / 64 bytes. A correction is building; acceptance and broader
   verification remain pending. Fresh snapshots remain separately covered.
   The corrected general batch is now accepted in `268add2`: [verified results](ARRAY_COLLECTION_BATCHES_2026_10_29.md)
   record 1,261 library passes, 411 integrations, all 150 test-size checksums,
   2,960 allocation pairs / 5,304 semantic checks without regressions, 15 extra
   contracts and 108 reviewed memory pairs. All 18 target groups have flat width
   scaling. Small and retained-snapshot peak-memory increases remain documented.
   Initial scalar-first journals and unsupported shapes retain fallback.
   [Scalar-first isolation](SCALAR_FIRST_BASELINE_2026_10_30.md) on `18ebd21`
   confirms a 269,824-byte owner-width penalty per added call for both mixed
   scalar-first and three-scalar sequences; the reordered collection-first
   control stays flat at 6,251 bytes. All 96 allocation observations and 144
   runtime checks pass. Scalar journal promotion and scalar-only batch admission
   are now accepted in `ac7f234`: [verified results](SCALAR_FIRST_BATCHES_2026_10_30.md)
   cover cold/small-owner controls, fallback allocation nonregression, all three
   runtimes and memory review. Mixed two-write updates, general selectors and
   other unsupported shapes still need characterization.
   [Post-promotion shape refresh](ARRAY_SHAPE_REFRESH_2026_10_30.md) now confirms
   a 277,440-byte width penalty for mixed two-write updates, initial strings and
   arithmetic selectors across widths 8/2,048 on `ac7f234`. All 192 allocation
   observations and 288 semantic checks pass. Literal-selector and string-last
   controls remain flat. Initial string/byte routing is now accepted in `94ad8f5`:
   [verified results](STRING_FIRST_BATCHES_2026_10_30.md) show string-first costs
   flat at 5,260 bytes per added call across widths 3/4/8/2,048, with no allocation
   regressions in target or historical controls. VM/JIT allocation
   results and all three runtimes' semantics are verified. Mixed two-write
   updates and arithmetic selectors remain separate open costs.
   [Mixed-pair baseline](MIXED_PAIR_BASELINE_2026_10_30.md) on `94ad8f5` now
   covers 12 ordered payload pairs at widths 2/3/8/2,048: 768 allocation
   observations and 1,152 semantic checks pass. All pairs retain a 278,256-byte
   width penalty between 2 and 2,048 elements; VM/JIT slopes agree. Compact mixed
   pair storage must preserve the existing frame layout and cheap homogeneous
   paths, with small-owner and snapshot controls retained.
2. Compound indexed arithmetic; primitive captured dependencies are now verified.
   [Baseline isolation](COMPOUND_CAPTURE_BASELINE_2026_10_29.md) separates
   captured reads, captured assignments and indexed arithmetic: each independently
   restores a 587,520-byte width penalty. The parameter-only direct-assignment
   control stays flat; 128 allocation observations and 192 semantic checks pass.
   The [earlier accepted-runtime refresh](compound-capture-e445-refresh-2026-10-29.json)
   on `e445f4a` passes the same 128/192 controls with unchanged slopes; the
   general array-batch fix is accepted separately in `268add2`.
   Primitive captured read/write/increment admission is now accepted in `30e1511`.
   [Verified results](CAPTURED_SCALAR_BATCHES_2026_10_29.md) record 1,262 library
   passes, 412 integrations, all 150 checksums, 576 allocation pairs / 864 semantic
   checks without regressions, 15 additional contracts and 72 reviewed memory pairs.
   All six target groups are flat; width-2,048 requested bytes per call fall from
   about 633 KB to 7 KB. The four compound-indexed variants retain their measured
   587,520-byte width penalty and remain the next fix. This is not an elapsed-speed claim.
   The [first compound candidate](COMPOUND_PUBLICATION_REJECTION_2026_10_29.md)
   is rejected for callback copying regressions. The corrected implementation is
   accepted in `7724558`: [verified results](COMPOUND_INDEXED_BATCHES_2026_10_30.md)
   record 1,263 library passes, 414 integrations, all 150 test-size checksums,
   656 allocation pairs / 984 semantic checks and 96 reviewed memory pairs. All
   eight compound targets have flat width scaling; callback allocation regressions
   are removed. General compound selectors retain fallback and remain for review.
3. Remaining capture traversal/materialization and oversized fallback; distinguish
   necessary work from avoidable costs before changing it.
   [Current-runtime refresh](REMAINING_CAPTURE_BASELINE_2026_10_30.md) passes
   80 allocation observations and 120 semantic checks. Prior allocation savings
   remain intact; instrumented sampling identifies exact capture-membership
   lookups as the next traversal experiment. This is not a speed or closure claim.
   Exact membership lookup optimization is now accepted in `18ebd21`:
   [verified results](CAPTURE_MEMBERSHIP_2026_10_30.md) retain exact host-edit
   validation while avoiding redundant hashes for unchanged name order. All 260
   allocation pairs, 390 semantic checks, 1,264 library tests, 414 integrations
   and 150 test-size checksums pass. Initial RSS increases and follow-up
   baseline variation remain documented. Required linear validation and broader
   materialization/fallback remain open; no elapsed-speed claim follows.
4. Collection representation review and final combined-candidate verification,
   including dense updates and fresh snapshots.

The nested-insertion, root replacement and eligible nested replacement cutoffs are
complete. Struct-literal construction and eligible direct root-array replacement
are also verified. The eligible general multiple-index batch cutoff is complete. Compound indexed arithmetic with callback-free selectors is now verified in `7724558`.
The **next cutoff** is isolation of remaining capture traversal/materialization
and unsupported collection shapes, including general selectors. Scalar-first journals are now verified separately. Preserve snapshots,
callbacks and error ordering and repeat applicable
regression and 150-checksum gates for further fixes. Sections 3 and 4 remain open.

The [first nested candidate](nested-insertion-rejection-2026-10-29.json) remains
rejected for 36 small-map allocation regressions. The subsequent
[conditional-path candidate](NESTED_CONDITIONAL_REJECTION_2026_10_29.md) remains
rejected for 12 total-allocation regressions. The accepted version checks actual
first-key admission separately from its reserve hint; it passes those controls
without relaxing comparison bounds. Root-insertion and nested-insertion candidate
numbers refer to separate experiments; their rejection evidence is not interchangeable.

Planning estimate: several more implementation/verification cycles, likely days
rather than hours; this is not measured remaining duration. Broad unchecked items
are not comparable in effort to individual checked tests or fixes. No defensible
percentage or remaining-versus-elapsed time ratio is available. Clean-host speed
comparisons remain pending separately; section 5 is outside this round.

Evidence: [allocation/collection report](ALLOCATION_COLLECTION_FIXES_2026_10_25.md),
`allocation-collection-2026-10-25.json`; runtime commits `e1ac6ba`, `787498d`.

- [x] Remove demonstrated avoidable hot-call allocations (10,000-call allocation control).
- [x] Reduce argument conversion/copying/temporary vectors (ordinary arguments move; legacy compatibility retained).
- [x] Reuse bounded frame storage with recursion/reentrancy isolation (32 buffers, at most 256 slots each; values cleared).
- [x] Cache ordinary parameter-slot/name metadata per immutable code object; public code edits invalidate the cache. Legacy receiver-field lookup remains unchanged.
- [x] Remove redundant mutable permission entries from imported environment snapshots: 64 calls fall from 92,129 requests / 13,840,564 bytes to 61,977 / 11,334,789.
- [x] Share immutable canonical native-default metadata in module environments; preserve overrides, snapshots and native-loop ownership. Runtime `8b4111b` passes 1,114 library tests, 204 integrations and 150 checksums. Seven-pair local priority medians are within 5%; see [native-default report](IMPORTED_NATIVE_DEFAULTS_2026_10_25.md).
- [x] Avoid captured-environment setup for proven parameter/local synchronous helpers (`2c342ef`). Per-added-call read traffic is flat across 0/128/2,048 unrelated bindings and 128/2,048-element arrays; the allocation regression gate fails on the baseline and passes on the candidate. 1,133 library tests, 286 integration tests and 150 test-size checksums pass; ignored profiling tests are listed in the report. General captured-binding and mutable-module write costs remain open. See [measured results and limits](CAPTURE_INDEPENDENT_HELPERS_2026_10_26.md).
- [x] Reuse bounded private capture-frame map/name storage (`6228389`), preserving public owners, callback publication and snapshots. Both allocation gates reproduce the baseline cost and pass on the candidate; write setup is flat across 0/128/2,048 unrelated bindings, and captured reads also cover immutable bindings. 1,138 library tests, 289 integrations, 150 test-size checksums and module-owner controls pass. Width copying and map traversal remain open. See [verification and tradeoffs](CAPTURE_FRAME_POOL_2026_10_26.md).
- [x] Reuse bounded recursive capture prefixes and global-map keys (`c0e45b3`), with cold/sparse-map growth protected. Depth-one setup is flat across 0/128/2,048 unrelated bindings; deeper and oversized owners retain bounded fallback costs. All 1,142 library tests, 289 integrations, 150 test-size checksums and helper/module allocation gates pass. The paired oversized gate rejects the earlier draft and passes the final candidate. See [results and limits](RECURSIVE_CAPTURE_POOL_2026_10_26.md).
- [x] Reuse suspended live-local name and binding-kind maps on recursive return
  (`e9e5695`). [Verified results](RECURSIVE_LOCAL_RESTORATION_2026_10_27.md)
  show requested bytes per added depth-two write call falling from 88,864 to
  4,302 at 128 retained locals, from 2,041,233 to 683,535 at 2,048, and from
  13,598,739 to 8,161,425 at 8,192. VM/JIT agree; all 96 allocation pairs and
  144 semantic cases pass without regressions. All 1,178 library tests,
  342 release integrations and 150 test-size checksums pass. Ordinary-local,
  cold, collection and publication controls pass. Pool bounds are unchanged;
  oversized activation, prefix materialization and traversal remain open.
- [x] Transfer private captured prefixes through direct synchronous live-local
  recursion (`ce923d1`). [Verified results](RECURSIVE_PREFIX_TRANSFER_2026_10_27.md)
  show constant added depth-two recursion cost across 0/128/2,048/8,192 locals:
  48 requests / 2,802 bytes for reads and 50 / 2,816 for writes, in VM/JIT.
  All 96 allocation pairs avoid regressions; all eight larger read/write groups
  strictly improve. The report preserves and explains the restoration-specific
  gate rejection and the separate prefix-scaling acceptance. All 1,179 library
  tests, 342 release integrations and 150 test-size checksums pass, with prior
  collection/cold/append controls preserved. Indirect recursion and oversized
  outer activation remain open; pool limits and value semantics are unchanged.
- [x] Transfer eligible suspended local prefixes through foreign callbacks (`65a58f2`).
  [Verified evidence](FOREIGN_LOCAL_PREFIX_TRANSFER_2026_10_27.md) covers seven
  widths through 16,384 locals, unchanged pool limits, six three-runtime
  regression contracts and all 150 test-size checksums. Recursion overhead is
  constant within each pool regime; the fixed boundary step remains. The original
  exact-equality rejection and rejected pool-bypass regression are preserved.
  Split-frame reentry and oversized outer activation remain open.
- [x] Transfer separately suspended captured locals and globals through shared-helper
  foreign reentry (`4f5087b`). [Verified evidence](SPLIT_OWNER_TRANSFER_2026_10_27.md)
  covers seven widths through 16,384 locals, 28 strict depth-four reductions,
  unchanged pool regimes and no tested allocation regressions. All 1,179 library
  tests, 348 release integrations and 150 test-size checksums pass. Oversized
  outer activation and general captured-environment materialization remain open.
- [x] Isolate oversized private lookup-table storage. [Component measurements](CAPTURE_KEY_LAYOUT_2026_10_27.md) show 5,444,778 to 1,250,474 requested bytes at 16,384 names using key/kind metadata, with unchanged request counts. The naive optional representation grows enclosing metadata from 16 to 24 bytes and is rejected as an integration design. Runtime integration is verified below.
- [x] Compact oversized captured-local lookup storage (`82bdef6`) without adding
  Environment fields or raising pool limits. [Verified evidence](COMPACT_CAPTURE_VIEWS_2026_10_27.md)
  shows per-added-call bytes falling from 5,447,495 to 1,253,191 at 16,384 locals,
  with unchanged requests and matching VM/JIT deltas. All 12 target groups improve;
  total, added-call, recursion and dense-write controls pass. A host-removal
  regression in the first candidate was reproduced and corrected before acceptance.
  All 1,184 library tests, 355 release integrations and 150 test-size checksums pass.
  Retained-name cloning and general capture/collection costs remain open.
- [x] Pack oversized private captured-local names into compact owned storage (`b4dfdfd`).
  [Verified results](COMPACT_CAPTURE_NAMES_2026_10_28.md) reduce 8,192-name read setup from 8,239 to 49 allocation
  requests and 624,208 to 313,012 requested bytes per added call. All 12 oversized
  groups improve without total, per-call or recursive allocation regressions.
  All 1,226 library tests, 400 integrations and 150 test-size checksums pass,
  with dense/cold/collection controls and unchanged pool limits. Slab copying,
  capture traversal and broader materialization remain open; no elapsed-speed claim.
- [x] Share validated oversized captured-local metadata (`6d28861`) with copy-on-write
  kind snapshots and stale-membership eviction after host scope edits.
  [Verified evidence](SHARED_CAPTURE_METADATA_2026_10_28.md) reduces 8,192-name
  read/write setup from 311,959/311,970 to 1,650/1,661 requested bytes per added call
  in VM/JIT. All 240 target/control allocation pairs, 360 associated semantic
  cases, 1,229 library tests, 403 integrations and 150 test-size checksums pass.
  A newly reproduced stale-owner cache regression was fixed before acceptance.
  The component memory tradeoff is 244,746 retained metadata bytes per 8,192-name
  owner; 32-owner local peak RSS means increase 2.9–3.7%. Pool limits and value
  semantics are preserved. Traversal, broader materialization and collection
  copying remain open; no elapsed-speed claim follows.
- [ ] Reduce remaining captured value-environment materialization without changing snapshots, writes, recursion or JIT lifetimes. [Component attribution](IMPORTED_ENVIRONMENT_ATTRIBUTION_2026_10_25.md) confirms map storage dominates requested bytes; copied names dominate requests. Four historical controls and 12 runtime snapshot/state checks passed. The [pre-fix refresh](IMPORTED_COST_REFRESH_2026_10_26.md) confirms remaining binding-count and mutable-module-array costs with 32 allocation and 60 semantic controls; general captured-binding and mutable-module write optimization remain open; eligible independent helpers are covered by the verified fix above. [Call-boundary isolation](MODULE_OWNER_ISOLATION_2026_10_26.md) now covers four collection shapes, one/eight writes, and once/fresh snapshots: 128 allocation and 192 semantic controls pass; three host-owner callback contracts also pass. Width-dependent copying remains per invocation, not per element write.
  [Shared-helper isolation](SPLIT_LOCAL_ISOLATION_2026_10_27.md) also confirms
  split-frame reentry costs on accepted `ce923d1`: 144 allocation cases and 216
  semantic runs pass; depth-four writes with 8,192 locals request 13,630,073
  bytes per added outer call. This is baseline evidence, not fix acceptance. A [current-runtime refresh](SPLIT_LOCAL_REFRESH_2026_10_27.md) confirms identical costs on `65a58f2` and adds explicit global/local ownership and helper-shadowing regression coverage.
- [x] Check return transfers and reusable callback state against ownership/import/callback regression suites; no new transfer defect established.
- [x] Check pooling/cache retention bounds and separately observe peak memory; no clean-host memory acceptance claim.
- [x] Test recursion, callbacks, exceptions, imports, captures and cancellation in the regression corpus (see exact revision limits in report).
- [x] Fix synchronous captured-state reentry, including callbacks, helper/generator suspension and VM bridges, while preserving independent snapshots. Runtime `45360b9` passes 48 focused cases, 259 integrations plus nine callback tests, 1,114 library tests, all 150 checksums, and helper/priority timing gates. See [verification and measured limits](CAPTURE_REENTRY_VERIFICATION_2026_10_26.md). Task ownership defects remain below.
- [x] Exclude ordinary global-helper parameters from async captured-write journals while retaining actual global and subsequent captured writes. Runtime `15a225d` passes all three runtimes; see [helper-journal verification](TASK_HELPER_JOURNAL_VERIFICATION_2026_10_26.md).
- [x] Preserve different-binding captured writes across overlapping ordinary task callbacks. Runtime `0b87ccb` passes all correctness gates and 15 local timing groups; see [verification and measured costs](ORDINARY_TASK_WRITE_VERIFICATION_2026_10_26.md).
- [x] Refresh task globals from each submission while retaining captured locals and separate module/callback namespaces (`2b13158`). See [current verification](TASK_GLOBALS_CALLBACK_LOADER_2026_10_26.md).
- [x] Avoid discarded callback module-loader discovery (`2e020ee`), retaining cache, capability and error behavior.
- [x] Preserve channel-ordered capture visibility across active task/caller reentry with public owner maps and independent snapshots intact. All 33 scenarios, 1,128 library tests, 283 distinct integration tests, linear append allocation controls and 150 test-size checksums pass. Interpreter regressions are enabled by default. See [owner-backed verification](LIVE_CAPTURE_OWNER_2026_10_26.md).

- [x] Project proven global-only captured reads (`c79010d`), retaining fallback for live locals, tasks, reentry and uncertain syntax. Oversized read setup falls from 8,231 requests/2,720,272 bytes to 16/1,086 per added VM call. All 1,146 library tests, 292 integrations, 150 test-size checksums and allocation gates pass. General/deeper captures and module-width copying remain open. See [results, rejected draft and limits](CAPTURED_READ_PROJECTION_2026_10_26.md).

- [x] Transfer private globals through direct synchronous global-only reentry with bounded scope reuse (`2b83f1e`). Depth-four setup is flat across 0/128/2,048 unrelated bindings; all 36 paired comparisons avoid request/byte growth. A stricter gate rejects the earlier shallow-call regression. All 1,146 library tests, 295 integrations and 150 test-size checksums pass. Indirect/local captures, publication traversal and module-width copying remain open. See [results and limits](RECURSIVE_GLOBAL_TRANSFER_2026_10_26.md).

- [x] Transfer suspended private globals through foreign-module callback reentry (`0cd6ed6`). Eligible global-only captures are flat at depths one/four across 0/128/2,048 bindings. All 36 cold comparisons and eight append controls match baseline allocation exactly. All 1,146 library tests, 301 integrations and 150 test-size checksums pass. The performance gate rejects the old baseline; an earlier frame-size regression was rejected. See [verification and remaining scope](FOREIGN_GLOBAL_TRANSFER_2026_10_27.md).

- [x] Reserve fresh captured-local maps once from actual key count (`aee15a2`). At 8,192 retained locals, VM/JIT allocate 2,637,372 fewer bytes and 12 fewer requests per added call; 0/128/2,048-local per-call traffic is unchanged. All 1,147 library tests, 301 integrations and 150 test-size checksums pass. The paired gate rejects the old runtime. Retained-name copying remains open. See [results and limits](LOCAL_MAP_RESERVE_2026_10_27.md).

- [x] Integrate bounded sparse scalar-array publication while preserving public
  pre-return values (`d337cfa`). [Runtime verification](INDEXED_PUBLICATION_RUNTIME_2026_10_27.md)
  proves flat added-call allocation across 128/2,048-element arrays: 16 of 128
  paired cases reduce bytes, 112 match, and none increase requests or bytes.
  1,157 library tests, 316 release integrations, 150 test-size checksums, 36 cold
  comparisons and eight append comparisons pass. The actual capture-enum layout
  remains unchanged on the local target; no elapsed-speed claim follows.
- [x] Defer repeated integer writes to one existing captured dictionary key
  (`bdb232d`) while preserving public-owner values and snapshots.
  [Runtime verification](DICTIONARY_PUBLICATION_RUNTIME_2026_10_27.md) shows flat
  added-call allocation across 128/2,048-key dictionaries: 16 of 128 matrix pairs
  improve, 112 match, none regress. The separate numeric-key gate passes all 12
  pairs, including exact baseline equality for fresh snapshots. All 1,162 library
  tests, 322 release integrations, 24 focused contracts and 150 test-size checksums
  pass; cold/append controls also pass. No elapsed-speed claim follows.
- [x] Defer eligible repeated scalar writes to one array element in captured
  struct/dictionary state (`ad68680`). [Runtime verification](NESTED_PUBLICATION_RUNTIME_2026_10_27.md)
  proves flat added-call allocation across 128/2,048-element rows with one/eight
  writes per helper. For 32 extra one-write calls at width 2,048, struct bytes
  fall from 9,241,184 to 303,680 and dictionary bytes from 9,243,072 to 305,824.
  All 128 module pairs and 72 additional controls avoid request/byte regressions,
  including competing direct-array/dictionary writes. All 1,177 library tests,
  339 release integrations and 150 test-size checksums pass. Real snapshots and
  fallback paths retain value semantics; this is not an elapsed-speed claim.
- [x] Batch up to four distinct scalar writes to one captured array row (`3d06ea2`),
  preserving snapshots, publication ordering and larger-batch fallback. Checked
  negative-index reads avoid speculative row copies; invalid root-as-index
  controls also pass. At 2,048 elements, two-entry struct-row updates fall from
  284,784 to 5,484 requested bytes per added call. All 384 target allocation pairs,
  576 associated semantic cases, historical allocation controls, 1,186 library
  tests, 355 release integrations and 150 test-size checksums pass. See
  [verified results and limits](NESTED_ROW_BATCH_2026_10_27.md).
- [x] Extend single-key captured dictionary publication to float, Boolean, null
  and mixed scalar types (`36b8006`). [Verified results](DICTIONARY_SCALARS_2026_10_27.md)
  show float/Boolean writes at 2,048 keys falling from 631,601 to 4,806 requested
  bytes per added call; integer controls stay unchanged. All 696 target/control
  allocation pairs, 1,044 semantic cases, 1,190 library tests, 358 release
  integrations and 150 test-size checksums pass. Capture storage layout, snapshots,
  exact float bits and escape behavior are preserved. Multiple-key/nonscalar and
  broader nested costs remain open; this is not an elapsed-speed claim.
- [x] Extend bounded row publication to array roots (`1c3acd4`), preserving
  positive/negative selector aliases, shared rows, snapshots and ordinary errors.
  [Verified results](ARRAY_ROW_PUBLICATION_2026_10_27.md) cover 624 new allocation
  pairs, 936 semantic cases and 30 diagnostic pairs. All 1,192 library tests,
  361 release integrations, prior controls and 150 test-size checksums pass.
  An earlier eight-case speculative-copy regression was reproduced and corrected;
  larger batches, deeper paths and multiple-key dictionaries remain open.
  The [post-array refresh](COLLECTION_GAP_AFTER_ARRAY_2026_10_27.md) verifies 240
  allocation measurements and 360 semantic checks on this runtime: array-owned
  rows are flat; multiple dictionary keys, deeper rows and nested dictionaries
  remain width-dependent.
- [x] Retain two existing canonical numeric dictionary keys inline, including
  compound assignments (`924e949`). [Verified results](DICTIONARY_PAIR_PUBLICATION_2026_10_28.md):
  target compound update bytes per added call fall from 639,941 to 13,146 at width
  2,048 and are flat across widths 2/128/2,048. All 48 target allocation pairs,
  72 semantic cases, 1,203 library tests, 379 release integrations and 150
  test-size checksums pass. Broader compound/nested paths remain open.
- [x] Defer eligible multiple-key captured dictionary writes (`19a1736`), including
  strings, bytes and new keys after promotion. [Verified results](DICTIONARY_MANY_PUBLICATION_2026_10_28.md)
  reduce two-write traffic at 2,048 keys from 634,379 to 7,788 bytes per added call.
  All 160 target pairs, 856 supplemental pairs, historical controls, 1,200 library
  tests, 376 release integrations and 150 test-size checksums pass. Four rejected
  drafts and exact diagnostic/ownership evidence are preserved. Compound indexed
  updates, nested dictionaries, deeper/larger rows and general captures remain open;
  fresh snapshots retain required copying. No elapsed-speed claim follows.
- [x] Isolate nested dictionary leaf costs on `924e949` across dictionary, struct
  and array owners with numeric/string keys: 144 allocation measurements and
  216 semantic checks pass. [Baseline and next acceptance gates](NESTED_DICTIONARY_ISOLATION_2026_10_28.md).
  This confirms remaining work; it does not mark the runtime optimization complete.
- [x] Defer one existing scalar dictionary leaf key under dictionary, struct and
  array owners (`2caf05f`). [Verified results](NESTED_DICTIONARY_PUBLICATION_2026_10_28.md):
  numeric dictionary-owner traffic falls from 632,275 to 4,804 bytes per added
  call at width 2,048. Eligible numeric/string-key costs are flat across widths.
  All 144 target pairs, 240 fallback pairs, 1,205 library tests, 382 release
  integrations and 150 test-size checksums pass. Broader paths remain open.
- [x] Extend nested dictionary publication to four scalar keys, including
  insertion after an existing-key write (`21a0907`).
  [Verified results](NESTED_DICTIONARY_BATCH_PUBLICATION_2026_10_28.md): two-key
  numeric dictionary-owner traffic falls from 633,076 to 5,562 bytes per added
  call at width 2,048. All 480 target pairs, two 240-pair fallback matrices,
  1,206 library tests, 385 release integrations and 150 test-size checksums pass.
  Five-key and deeper-path costs remain open.
- [x] Isolate larger nested dictionary batches on `21a0907`: 384 allocation
  measurements and 576 semantic cases cover 4/5/16/64 writes across owners, key
  forms and snapshot policies. [Baseline and spill acceptance gates](NESTED_DICTIONARY_WIDE_ISOLATION_2026_10_28.md)
  confirm a width-dependent copy at the fifth key; the runtime fix remains open.
  The [first sparse overflow candidate was rejected](NESTED_DICTIONARY_SPILL_REJECTION_2026_10_28.md):
  80 escape/fallback allocation pairs regress by 280 bytes per warmed call.
  Runtime changes are reverted; full-key snapshot tests and the five-write
  fallback gate are retained. Overflow optimization remains open.
- [x] Extend proven synchronous nested scalar dictionary batches beyond four
  keys (`a5d07bb`). [Verified results](NESTED_DICTIONARY_SPILL_PUBLICATION_2026_10_28.md) show
  five-key numeric dictionary traffic falling from 634,213 to 6,979 bytes per
  added call at width 2,048. All 384 target pairs, fallback/error/branch controls,
  1,213 library tests, 388 release integrations and 150 test-size checksums pass.
  Two rejected candidates are preserved; uncertain bodies and broader paths
  retain fallback. Sections 3/4 remain open.
- [x] Isolate deeper array/dictionary leaf updates on `a5d07bb`: 384 allocation
  measurements and 576 semantic checks cover three root owners, depths 2/4,
  one/eight writes and initial/fresh snapshots. [Verified baseline](DEEP_COLLECTION_ISOLATION_2026_10_28.md)
  reproduces per-call width penalties of 261,120 bytes for arrays and 587,520
  for dictionaries, independent of write count. The eligible deeper-path fix is verified below.
- [x] Extend scalar publication/read handling through unique deeper collection
  prefixes (`ca3403d`). [Verified results](DEEP_COLLECTION_PUBLICATION_2026_10_28.md) show
  width-2,048 deep array/dictionary one-write traffic falling from 285,988/634,172
  to 5,259 bytes per added call. All 384 target pairs, 512 deep fallback pairs,
  78 diagnostic pairs, 1,216 library tests, 391 release integrations and
  150 test-size checksums pass. Larger batches and broader capture costs remain open.
- [x] Refresh larger row-batch isolation on `ca3403d`: [verified baseline](ROW_OVERFLOW_ISOLATION_2026_10_28.md)
  covers 288 allocation pairs, 432 semantic cases and 54 permanent regression
  combinations. Both loop and explicit writes above four elements retain a
  261,120-byte per-call width penalty. The measured overflow storage and admission fix is verified below.
- [x] Retain larger scalar row batches in private overflow storage (`c83ee4b`).
  [Verified results](ROW_OVERFLOW_PUBLICATION_2026_10_28.md) cover both explicit writes and bounded counted
  loops: five writes into a width-2,048 dictionary-owned row fall from
  286,036 to 6,922 bytes per added loop-helper call. The four-entry path stays
  unchanged. All 288 target pairs, 832 larger escape pairs, 1,222 library tests,
  397 release integrations and 150 test-size checksums pass. General/deeper loops
  and broader capture/collection costs remain open.
- [x] Isolate insertion, nonscalar replacement and loop admission factors on
  `6d28861`. [Verified characterization](COLLECTION_FACTOR_ISOLATION_2026_10_28.md)
  covers 192 allocation observations and 288 semantic checks. Insertion,
  nonscalar and compound/captured-counter loops retain a 587,520-byte width
  penalty; direct parameter-only eight-key assignments are already flat at
  7,307 bytes per added call. All 192 same-path comparisons match `4d11952`.
  The broader costs below remain open; dynamic bounds alone are not the cause.
- [ ] Reduce remaining module-width copying for insertion, nonscalar updates,
  general/deeper loops and dense/escaping update patterns. The [current refresh](OWNERSHIP_COST_REFRESH_2026_10_28.md)
  on `4d11952` verifies flat initial-snapshot added-call costs for all ten historical
  small-update shapes across widths 2/128/2,048 (240 allocation observations,
  360 semantic checks). Those small-update examples are fixed; this broader item
  remains open for cases outside that matrix. The
  [post-scalar refresh](COLLECTION_GAP_REFRESH_2026_10_27.md) verifies 240 allocation
  measurements and 360 semantic cases on `36b8006`: single-key scalar and bounded
  row costs are flat, while multiple keys, array-owned/deeper rows and nested
  dictionaries still scale with width. The
  [earlier accepted-runtime isolation](COLLECTION_GAP_ISOLATION_2026_10_27.md)
  verifies 240 allocation measurements and 360 semantic cases on `82bdef6`.
  Two-key and floating-point dictionary writes request 631,177 bytes per added
  call at width 2,048; two-element struct-row writes request 284,436 bytes.
  Fresh-snapshot controls remain separate. The four-entry row integration above
  addresses the bounded row case; larger batches and the other costs remain open.
  [Scalar dictionary isolation](DICTIONARY_SCALAR_ISOLATION_2026_10_27.md)
  adds 120 allocation measurements and 180 semantic checks: float, Boolean,
  null and mixed-type writes scaled with dictionary width before the scalar fix, while the
  eligible integer control stays flat. These are historical baseline measurements; the scalar fix above now addresses this single-key case.
  The
  [earlier model](INDEXED_PUBLICATION_JOURNAL_2026_10_27.md),
  [nested-row attribution](NESTED_ROW_ATTRIBUTION_2026_10_27.md) and
  [test-only storage candidate](NESTED_PUBLICATION_CANDIDATE_2026_10_27.md) remain
  historical design evidence. The accepted runtime report retains its rejected
  first integration and the subsequent ownership/interference corrections.

Gate: the reproduced call/frame costs are reduced with bounded retention.
[Indirect retained-local isolation](INDIRECT_LOCAL_ISOLATION_2026_10_27.md) reproduces remaining foreign-callback prefix copying with 144 allocation and 216 semantic cases on `ce923d1`; eligible same-owner and split-owner foreign reentry are now addressed by [foreign prefix transfer](FOREIGN_LOCAL_PREFIX_TRANSFER_2026_10_27.md) and [split-owner transfer](SPLIT_OWNER_TRANSFER_2026_10_27.md). Broader capture materialization remains open.

The [ownership refresh](OWNERSHIP_COST_REFRESH_2026_10_28.md) measured oversized
capture setup at 311,959 read/311,970 write bytes per added call with 8,192 bindings.
[Shared metadata verification](SHARED_CAPTURE_METADATA_2026_10_28.md) now reduces
that eligible path to 1,650/1,661 bytes in VM/JIT, with an explicit retained-memory
tradeoff. General traversal, exceptional fallback and broader materialization remain open.

Section 3 remains open for capture-map traversal, captured-local prefix materialization, oversized-owner fallback and module collection detachment; native preload copying, eligible direct/foreign synchronous global and local reentry (including separately suspended owners) and the reproduced task capture ownership defects are fixed. [Indirect recursion isolation](INDIRECT_RECURSION_ISOLATION_2026_10_27.md) records the foreign-callback baseline addressed above. [Retained-local isolation](LOCAL_CAPTURE_ISOLATION_2026_10_27.md) records the oversized capture baseline at 8,192 genuinely retained locals. The [fresh-map reserve fix](LOCAL_MAP_RESERVE_2026_10_27.md) removes repeated table growth; retained-name copying and the other costs listed here remain open. The [shared-name component experiment](SHARED_CAPTURE_NAMES_2026_10_28.md) finds repeated-call savings but cold-owner allocation regressions in 30 of 55 boundary groups; unconditional String-to-Arc key conversion is rejected, with no runtime change. Follow-up evidence:
[imported setup and bytes report](IMPORTED_SETUP_AND_BYTES_2026_10_25.md),
`imported-setup-and-bytes-2026-10-25.json`; final 150 test-size checksums pass.

## 4. Collection and access fixes (verified global-write fix; broader review open)

- [x] Profile dictionary → array → struct access (prior diagnosis and 270 final ownership controls).
- [x] Preserve unique backing storage across repeated global array/string-dictionary mutation, including scoped compiler-emitted stores.
- [x] Preserve real snapshots through copy-on-write detachment (one snapshot and fresh-per-write controls).
- [x] Reuse valid depth evidence without repeated full-container scans in the changed path.
- [x] Verify changed-path invalidation for external growth, root replacement, deep-value rejection/repair and aliases; existing nested-path suites pass.
- [x] Check sustained dictionary deletion, stack and queue costs (256/1,024/4,096 diagnostic operations; linear scaling).
- [x] Review observable iterator snapshot costs with source mutation; first detachment is required in the tested case, so no owner was removed.
- [x] Measure representative byte conversion, UTF-8, indexing and iterator scaling; reserve bytes(array) output once, preserve ordered validation and snapshots.
- [x] Borrow successful dictionary lookup keys and defer missing-key diagnostics
  (`bb0bd0e`). [Verification](DICTIONARY_READ_RUNTIME_2026_10_27.md) removes
  key-length allocation growth: 128 extra lookup-loop iterations now request
  768 bytes for both 8/1,024-byte keys, versus 4,864/526,464 before. The 16-pair
  targeted gate, 128-pair module controls, 12-pair numeric-key controls, 1,163
  library tests, 328 integrations and 150 test-size checksums pass. Cold/append
  controls also pass; no elapsed-speed claim follows.
- [x] Remove the two nested-assignment key copies in validation and mutation
  (`868dfd8`). [Verification](ASSIGNMENT_KEY_RUNTIME_2026_10_27.md) shows 128 extra
  writes request 43,136 bytes for both 8/1,024-byte keys, versus 45,184/305,280
  before; VM/JIT agree. All 336 release integrations, 18 direct nested-assignment
  runs and 150 test-size checksums pass. The 128-pair module matrix has 32 byte
  reductions, 96 matches and no regressions; numeric/cold/append controls pass.
  Canonical integer keys, insertion, snapshots and error atomicity are preserved.
  Nested-row publication and broader collection costs remain open; no speed claim.
- [x] Borrow VM nested-write opcode metadata and keep short resolved paths in
  operation-local inline storage (`4d11952`), retaining deeper-path fallback.
  [Verified results](NESTED_PATH_STORAGE_2026_10_28.md) remove two requests and
  320 requested bytes per nested VM write at widths 128/2,048. Unchanged priority
  workloads show 82.356%/91.165% fewer requested MergeSort bytes in VM/JIT;
  NeuralNet falls 31.671%/33.961%. All 1,226 library tests, 403 integrations,
  150 test-size checksums and targeted controls pass. The reusable gate rejects
  the unchanged baseline; tiny QuickSort/BrainfuckArray differences overlap in
  four paired observations. These are allocation results, not elapsed-speed wins.
- [ ] Broaden specialized representation review when profiles identify additional costs; representative controls are not universal coverage. The [chunked-owner experiment](CHUNKED_OWNER_PROTOTYPE_2026_10_26.md) passed 63 numeric snapshot cases but rejects chunking plus contiguous publication: all 27 matched cases allocate more. No runtime integration or closure of this item.
- [x] Add scaling checks for accidental quadratic behavior, including actual compiler-emitted loops rather than hand-built opcodes alone.
- [x] Observe peak memory separately from allocation traffic; clean-host acceptance remains pending.
- [x] Implement the user-approved root-after-index rule for direct indexed writes in VM/JIT. Runtime `3a9cbe4` passes 30 targeted cases, actual native execution proof, 206 integration tests, 1,114 library tests and 150 test-size checksums; paired allocation and timing gates passed. See [verification and limitations](INDEX_ROOT_ORDER_VERIFICATION_2026_10_26.md). Subsequent captured-state reentry and ordered-visibility fixes are verified in section 3; imported environment materialization remains open.

Gate: changed global paths have better allocation scaling with ownership, errors,
bounds and depth checks intact. Broader collection representation costs and imported
environment materialization remain open; the reproduced captured-state reentry
defects have separate verified fixes in section 3.

## 5. Compiled execution (not started in this round)

- [ ] Refresh per-benchmark native execution/fallback evidence.
- [ ] Identify hot unsupported operations and expensive transitions.
- [ ] Extend compiled execution for demonstrated hot paths.
- [ ] Reduce measured guard/dispatch overhead safely.
- [ ] Preserve integer/Boolean/float/error semantics.
- [ ] Verify branches, recursion, imports, captures and unwinding.
- [ ] Tune JIT admission against small-workload compilation costs.
- [ ] Measure compilation time, code memory and cache lifetimes.
- [ ] Keep JIT opt-in pending reliability/portability acceptance.
- [ ] Consider architecture changes only if profiles justify them.

Gate: measured speed improvements, not just additional compilable functions.

## 6. Per-fix regression gate

The report records this round's evidence. These reusable gates remain requirements
for subsequent fixes; unchecked production/platform gates are not implied complete.

- [ ] Reproduce the original issue before modifying it.
- [ ] Add a test that detects its return.
- [ ] Test VM, interpreter and JIT.
- [ ] Test unique ownership and shared snapshots.
- [ ] Test relevant errors, cancellation and capabilities.
- [ ] Run targeted tests, formatting and required builds.
- [ ] Broaden regression tests according to the affected runtime surface.
- [ ] Compare matching release builds before/after.
- [ ] Investigate repeatable slowdowns above 5% on a controlled host. Loaded-laptop timings are diagnostic only; preserve correctness and allocation/scaling gates locally.
- [ ] Reject unreviewed tradeoffs that damage other workloads.
- [ ] Commit/push each verified logical change separately.
- [ ] Resolve parallel-callback global-write semantics against the canonical
  contract before changing behavior; escalate a real ambiguity for user review.

## 7. All-50 verification

Each family below requires exact input/checksum, build identity, status, elapsed
time, resource evidence and limitations for every case and runtime.

| Benchmark | Production VM | Production interpreter | Production JIT |
| --- | --- | --- | --- |
| Binarytrees::Obj | [ ] | [ ] | [ ] |
| Binarytrees::Arena | [ ] | [ ] | [ ] |
| Brainfuck::Array | [ ] | [ ] | [ ] |
| Brainfuck::Recursion | [ ] | [ ] | [ ] |
| Matmul::Single | [ ] | [ ] | [ ] |
| Matmul::T4 | [ ] | [ ] | [ ] |
| Matmul::T8 | [ ] | [ ] | [ ] |
| Matmul::T16 | [ ] | [ ] | [ ] |
| Base64::Encode | [ ] | [ ] | [ ] |
| Base64::Decode | [ ] | [ ] | [ ] |
| Json::Generate | [ ] | [ ] | [ ] |
| Json::ParseDom | [ ] | [ ] | [ ] |
| Json::ParseMapping | [ ] | [ ] | [ ] |
| CSV::Parse | [ ] | [ ] | [ ] |
| Etc::Sieve | [ ] | [ ] | [ ] |
| Etc::TextRaytracer | [ ] | [ ] | [ ] |
| Etc::NeuralNet | [ ] | [ ] | [ ] |
| Etc::CacheSimulation | [ ] | [ ] | [ ] |
| Etc::GameOfLife | [ ] | [ ] | [ ] |
| Etc::Words | [ ] | [ ] | [ ] |
| Etc::LogParser | [ ] | [ ] | [ ] |
| Template::Regex | [ ] | [ ] | [ ] |
| Template::Parse | [ ] | [ ] | [ ] |
| Sort::Quick | [ ] | [ ] | [ ] |
| Sort::Merge | [ ] | [ ] | [ ] |
| Sort::Self | [ ] | [ ] | [ ] |
| Graph::BFS | [ ] | [ ] | [ ] |
| Graph::DFS | [ ] | [ ] | [ ] |
| Graph::AStar | [ ] | [ ] | [ ] |
| Hash::SHA256 | [ ] | [ ] | [ ] |
| Hash::CRC32 | [ ] | [ ] | [ ] |
| Calculator::Ast | [ ] | [ ] | [ ] |
| Calculator::Interpreter | [ ] | [ ] | [ ] |
| Maze::Generator | [ ] | [ ] | [ ] |
| Maze::BFS | [ ] | [ ] | [ ] |
| Maze::AStar | [ ] | [ ] | [ ] |
| CLBG::Fannkuchredux | [ ] | [ ] | [ ] |
| CLBG::Mandelbrot | [ ] | [ ] | [ ] |
| CLBG::Nbody | [ ] | [ ] | [ ] |
| CLBG::Spectralnorm | [ ] | [ ] | [ ] |
| Compress::BWTEncode | [ ] | [ ] | [ ] |
| Compress::BWTDecode | [ ] | [ ] | [ ] |
| Compress::HuffEncode | [ ] | [ ] | [ ] |
| Compress::HuffDecode | [ ] | [ ] | [ ] |
| Compress::ArithEncode | [ ] | [ ] | [ ] |
| Compress::ArithDecode | [ ] | [ ] | [ ] |
| Compress::LZWEncode | [ ] | [ ] | [ ] |
| Compress::LZWDecode | [ ] | [ ] | [ ] |
| Distance::Jaro | [ ] | [ ] | [ ] |
| Distance::NGram | [ ] | [ ] | [ ] |

- [x] Recheck all 150 test-size combinations after these optimizations (`c83ee4b`; row-overflow-publication-2026-10-28.json). Repeat for later runtime changes.
- [ ] Complete unchanged production in VM, interpreter and JIT separately.
- [ ] Preserve timeout/failure/cancellation distinctions.
- [ ] Use final-candidate evidence rather than substitute old passes.
- [ ] Investigate resource growth/scaling failures.
- [ ] Retain auditable raw outputs and identities for every pass.

Gate: all 150 production checksums verified; no reduced-input substitutions.

## 8. Competitive and platform acceptance

- [ ] Repeat Goose/Rust comparisons on the same clean host.
- [ ] Report every case, loss and unresolved measurement.
- [ ] Report distributions/variability rather than best timings.
- [ ] Include startup and memory costs.
- [ ] Track targets against the faster comparator: 2×, 1.25×, repeatable wins.
- [ ] Treat targets as goals, never guaranteed outcomes.
- [ ] Finish macOS and rerun affected Linux/Windows gates.
- [ ] Verify relevant supported architectures/features.
- [ ] Verify release artifacts and local runner integration.
- [ ] Prepare remaining maintainer questions without contacting Kostya.
- [ ] Obtain separate authorization for upstream submission or publication.

## Execution policy

User clarification, 2026-10-26: local background load changes continuously.
Treat local elapsed-time comparisons as diagnostics, not strict percentage gates
or competitive rankings. Keep correctness, checksums, ownership and allocation
checks active. Full speed comparisons remain pending on a clean server.


Profile and reproduce the slow cases before another broad production sweep.
Use targeted production checks after meaningful fixes, and full sweeps at
milestones. No benchmark-specific native kernels, altered algorithms, weakened
checks, or performance claims based on interrupted/failed samples.
