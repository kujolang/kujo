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
| 1. Native array/field access | Guarded direct access or fused operations with fewer helper crossings; errors, snapshots and invalidation preserved | Tactical helper-view experiment rejected; direct-native architecture remains open |
| 2. Known-callee calls/recursion | Reduce measured VM marshalling and root traffic; preserve dynamic binding, frame limits, callback state and errors | 51/51 baseline call-scaling controls pass; attribution and implementation pending |
| 3. JIT admission/worker reuse | Recover cold compilation losses while retaining long-run native coverage; reuse code only under proved ownership/isolation | Pending |
| 4. VM instruction/value movement | Measure cloning/dispatch; implement supported tactical reductions; evaluate compact representation separately | Pending |
| 5. Layout/COW/validation | Attribute actual copied bytes; remove avoidable work; evaluate shape-indexed structs/homogeneous storage without weakening snapshots | Pending |
| 6. Libraries/callbacks/bytes | Identify conversion/comparison costs; optimize confirmed paths with exact ordering, malformed-input and Unicode contracts | Pending |
| 7. Startup/reusable compilation | Measure each frontend/setup phase; reduce confirmed redundant work; evaluate lazy/reusable artifacts with source/ABI invalidation | Pending |
| 8. Memory lifetime | Complete separate lifetime candidate's resource/platform gates; measure retained code and live heap separately | Pending, candidate separate |
| Port equivalence | Audit all 50 algorithms/work; isolate port improvements from runtime wins; strengthen structural oracles | Representative research, LZW and graph controls; [maze source review](MAZE_PORT_SOURCE_AUDIT_2026_11_02.md) partial; full audit pending |
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

The tactical array-region experiment is [closed and rejected](ARRAY_REGION_REJECTED_2026_11_02.md).
Its first candidate and sole revision passed their correctness gates but did not
establish repeatable target gains. The revision's 504-sample sweep consistently
slowed byte/dictionary fallback paths despite array microbenchmark gains. The
unaccepted runtime/changelog edits are removed; all 1,390 frozen identities again
match the accepted baseline. Semantic fixtures and all negative evidence remain.
Do not begin a third tactical revision or mark the native-access area complete.

The next bounded experiment targets known-callee owned calls and remaining VM
marshalling. Use the frozen accepted CLI: local target/release artifacts still
contain the rejected candidate until explicitly rebuilt. Preserve value semantics,
dynamic rebinding, errors and frame/cancellation contracts. Existing positional
arguments, argument buffers and root pools are already implemented.

[Call-scaling entry controls](probes/NATIVE_CALL_SCALING.md) pass 51/51 cases on
the accepted CLI across all three runtimes. They cover scalar arities 0–4 and
array arities 1–4 at three widths, with snapshots retained. These are correctness
entry controls, not measured speed or proof of the future optimized call path.

Independent-session, production, memory and clean-host gates remain open for the
broader campaign. No candidate from the read-region experiment is accepted.

### Entry controls established

`probes/native-region-contracts.kujo` passes on the frozen accepted CLI in VM,
interpreter and JIT modes. It covers repeated same-array reads, negative indices,
callee replacement with a retained snapshot, scalar/owned/type transitions,
nested result mutation, Unicode fallback, invalid accesses and post-error reads.
The JIT measurement receipt records native participation; it does not establish
future region-specialization coverage or a timing win. Runs overlap the baseline
build and are correctness-only. The interpreter CLI rejects `--measurements`, so
its verification uses exact stdout and exit status instead.

`native-region-contracts-2026-11-02.json.gz` retains source, collector, three
successful runtime outcomes and the supported VM/JIT metric receipts, with every
embedded hash verified. All implementation and candidate acceptance gates remain
open. Baseline build/profiling state is in
`.audit-evidence/native-performance-campaign-2026-11-02/state.json`; poll the live
process/session before attempting any restart.

### Research recommendation corrected during source review

The area-2 suggestion to reuse the first pre-call `DEBUG_JIT_CALL` lookup is
**not admitted** under behavior preservation. The existing
[debug environment disposition](DEBUG_CALL_ENVIRONMENT_2026_11_01.md) explicitly
rejects it: concurrent writers can change the second pre-call diagnostic
independently, even when the post-callback read remains. This is stronger evidence
than the supplied report's recommendation. Preserve all existing observation
points unless a separate contract change is explicitly approved. The broader
known-callee/marshalling experiment remains open; a debug-lookup cleanup would not
complete it in any event.

### First candidate in verification: borrowed array views

The unaccepted candidate uses an activation-local array-buffer view at repeated
local reads. It retains no extra Arc and checks bounds and index kinds on every
access. Rebinding the owning local, mutations, calls and unknown instructions
invalidate the view; dynamic arithmetic operators invalidate on their owned
branch because they can call user code. A whitelist fails closed for new opcodes.
The existing generic path supplies non-array/non-integer and error behavior.

This candidate still calls a Rust element-read helper. It tests whether removing
repeated root-slot validation and array extraction is material; it does **not**
prove direct generated scalar loads, complete native collection specialization,
or Rust/Goose parity. If whole-workload gains are absent, reject this tactic rather
than declaring the broader area complete. The frozen accepted library is retained
alongside its symbolized harness for subsequent paired diagnostics.

New verification covers write/append/call/replacement transitions, retained
snapshots and loop reads in all engines. Test-only counters require generated
region participation and one validation for repeated uninterrupted reads. The
expanded semantic fixture passes on the accepted CLI in all three engines.
Candidate tests/build, negative-index and mixed fallback path coverage, paired
timing, memory checks and all-50 gates remain pending. No speed claim or accepted
runtime commit exists. Candidate hashes and test process receipts live under
`.audit-evidence/native-performance-campaign-2026-11-02/region-*`.
