# Native performance campaign — 2026-11-02

Authorized by the user's goal, “do All of that,” following review of the supplied
performance investigation. This is a new campaign; the accepted checkpoint and
completed sections 3/4 cutoff stay closed. No tags, publication or Kostya contact.
The separate VM-owned code-lifetime candidate remains excluded until its own
acceptance gates pass. The objective includes all eight areas and the validation
roadmap below; the first three experiments are a delivery sequence, not a reduced
completion definition. Rust/Goose parity is an objective to measure, not a promised
outcome or a reason to weaken Kujo semantics.

## Immutable entry evidence

- Accepted runtime: `782c712d`; checkpoint: `7fd4f499`; comparison: `66a59e4e`.
- Baseline CLI SHA-256: `3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
- [Checkpoint results](CHECKPOINT_COMPARISON_2026_11_02.md) and
  [release gates](CHECKPOINT_REVIEW_2026_11_02.md) remain authoritative historical evidence.
- Supplied research: local `.audit-evidence/performance-investigation-2026-11-02/REPORT.md`,
  with raw evidence in the adjacent `evidence.json.gz`. Report claims require
  appropriate source/raw-evidence verification before implementation decisions.
- No runtime source changes at campaign entry. About 4.1 GiB free disk; no other
  cargo/rustc build process observed. Existing dependency cache may be reused;
  do not delete user files or launch overlapping builds and timing runs.

## Full work ledger

| Area | Required outcome | State |
| --- | --- | --- |
| 1. Native array/field access | Guarded direct access or fused operations with fewer helper crossings; errors, snapshots and invalidation preserved | Attribution/design in progress |
| 2. Known-callee calls/recursion | Reduce measured VM marshalling and root traffic; preserve dynamic binding, frame limits, callback state and errors | Pending |
| 3. JIT admission/worker reuse | Recover cold compilation losses while retaining long-run native coverage; reuse code only under proved ownership/isolation | Pending |
| 4. VM instruction/value movement | Measure cloning/dispatch; implement supported tactical reductions; evaluate compact representation separately | Pending |
| 5. Layout/COW/validation | Attribute actual copied bytes; remove avoidable work; evaluate shape-indexed structs/homogeneous storage without weakening snapshots | Pending |
| 6. Libraries/callbacks/bytes | Identify conversion/comparison costs; optimize confirmed paths with exact ordering, malformed-input and Unicode contracts | Pending |
| 7. Startup/reusable compilation | Measure each frontend/setup phase; reduce confirmed redundant work; evaluate lazy/reusable artifacts with source/ABI invalidation | Pending |
| 8. Memory lifetime | Complete separate lifetime candidate's resource/platform gates; measure retained code and live heap separately | Pending, candidate separate |
| Port equivalence | Audit all 50 algorithms/work; isolate port improvements from runtime wins; strengthen structural oracles | Representative research only; remaining audit pending |
| Production and competition | All-50 test and production checks; repeat frozen Kujo/Rust/Goose comparisons with failures/timeouts retained | Pending |
| Clean host | Dedicated-host reproduction, independent sessions and resource ceilings | Pending; no dedicated host supplied |
| Architecture | Typed native regions, shapes/storage, escape/ownership analysis, code owners/reuse and compact VM evaluation | Pending; narrow experiments precede migration |

## First delivery sequence

1. **Native collection access.** Start with callback-free, read-only regions and
   explicit guard lifetime. Verify existing scalar fast paths and dense-view
   mechanisms before introducing another representation. Measure helpers/root
   lookup and generated operations. Avoid retaining pointers across callbacks,
   mutation, root recycling, exceptions or storage changes.
2. **Known-callee owned calls.** Establish baseline marshalling/root/environment
   traffic. Specialize guarded positional calls; keep generic fallback and
   rebinding visibility. A diagnostic-lookup cleanup alone does not complete this
   experiment.
3. **Compilation economics.** Reproduce threshold and per-task duplication;
   measure compile time separately from execution. Use general policies, never
   benchmark names or moving compilation outside the official timer. Prove task
   isolation before sharing code or reusing a VM.

Each experiment permits one tactical implementation and at most one revised
hypothesis before an explicit accept/reject review. A rejected experiment remains
recorded and its broader area remains open where requirements are unmet. Do not
combine speculative changes into an unattributable patch.

## Verification gates for every candidate

- [ ] Source/binary/input identities frozen; pre-change cost and correctness evidence retained.
- [ ] Current accepted-path attribution distinguishes native, helper, VM and compiler work; unresolved frames remain unknown.
- [ ] Structural oracle and intended-path evidence defined before changing source.
- [ ] VM, interpreter and JIT parity, including snapshots, aliases, dynamic rebinding, invalid indices, error order, overflow/depth and callbacks.
- [ ] Native path actually executes; no timing win from accidental fallback or omitted work.
- [ ] Allocation traffic, live roots, retained memory and executable code tracked separately where relevant.
- [ ] Targeted tests and required formatting/build/integration/release checks pass.
- [ ] All 50 unchanged test-size checksums pass in all three runtimes.
- [ ] Paired uninstrumented measurements and independent session confirm improvement; unexplained regressions investigated, not hidden.
- [ ] Production-size outcomes preserve deadlines, cancellations, failures and memory-limit outcomes.
- [ ] Evidence reviewed; small meaningful commit pushed; rejected versions remain identifiable.

The report's 5% regression threshold triggers investigation; it is not permission
to accept smaller regressions blindly. Dynamic/value semantics and capabilities
remain intact. Use measured self-time coverage for Amdahl calculations; do not
turn event counts, waiting-thread samples or differences of medians into CPU-time
percentages. Cold and warm performance are separate outcomes. Clean-host claims
remain pending until that evidence exists.

## Current next action

Rebuild a source-bound accepted library from the existing cache, then link the
existing symbolized diagnostic harness and sample after warmup. The accepted CLI
is stripped; sampling it alone cannot resolve the runtime/helper boundaries the
first experiment needs. The harness is diagnostic, not a replacement competitive
binary. Confirm all outputs and source identity; collect uninstrumented CLI
before/after timings separately. No runtime optimization is accepted yet.
