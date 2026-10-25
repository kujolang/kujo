# Kujo performance implementation plan

Aim: make unchanged LangArena workloads competitive with Goose and Rust while
preserving Kujo's language contracts. Beating both languages on every workload is
a target to test, not a guarantee. This document explains the execution order;
[performance-plan.json](performance-plan.json) tracks all 50 cases and evidence.

## Current evidence and limits

- **Worker JIT policy remains a gap.** The
  [worker coverage probe](worker-jit-coverage-2026-10-24.json) on frozen `4e42bd2`
  performs two identical integer-kernel calls per variant. Direct execution
  records two native function entries; synchronous and async `parallel_map`
  record none, despite `--jit`, and all outputs match. Mapper VMs start with JIT
  disabled; cancellable language tasks explicitly disable it. This can limit
  threaded workloads, but the probe does not quantify its production speed cost.
  Propagate execution policy explicitly and establish cancellation checks in
  native loops/calls before enabling cancellable workers. Required gates include
  cancellation latency and admission release, capability inheritance, isolated
  globals, snapshots, ordering, errors, nested tasks and VM/interpreter/JIT parity.
  Do not enable JIT globally or remove cancellation to improve benchmark scores.

- **Consumed native call operands (`4e42bd2`).** [Evidence](CONSUMING_CALLS_2026_10_24.md)
  records zero isolated marshalling allocations and QuickSort JIT unique requests
  14,638→7,322. Borrowed APIs stay reusable; nested ownership and errors pass
  1,097 library and 40 integration tests. The
  [frozen release audit](CONSUMING_RELEASE_2026_10_24.md) verifies 150/150
  test-size checksums, 28 timing samples and eight peak-memory observations.
  JIT QuickSort's median paired speedup over `aa30071` is 1.0363×, with all
  seven pairs favorable. CPU speed limits changed 55→71%; this is provisional
  local evidence. Production, controlled competitive and platform gates remain
  open, and two RSS samples per variant do not establish a memory improvement.

- **Builtin argument reuse (`aa30071`).** [Evidence](BUILTIN_ARGUMENT_POOL_2026_10_24.md)
  records JIT QuickSort unique allocations 21,954→14,638 and requested bytes
  1,084,600→89,624. 1,095 library and 125 integration tests pass, including a
  warmed zero-allocation builtin bridge regression. [Frozen release audit](BUILTIN_POOL_RELEASE_2026_10_24.md)
  passes 150/150 test-size checksums, 28 paired timing and 8 memory samples.
  Compared with 93e7caa, median paired speedup is 1.0265× VM and 1.0454× JIT;
  CPU speed limits changed 100→71%, so controlled acceptance remains open.
  Production/platform/competitive verification is still pending.

- **Callback argument buffers (`1174b7b`).**
  [Allocation and semantic evidence](ARGUMENT_POOL_2026_10_24.md) records about
  50% fewer requests and 88% fewer cumulative requested bytes for the unique
  8,192-element JIT QuickSort probe. Bounded empty-buffer reuse preserves active
  call isolation, argument order and snapshots. 1,094 library and 44 targeted
  integration tests pass; release, production and peak-memory gates remain open.

- **Root storage identity (`9f401eb`).**
  [Integrated evidence](STORAGE_IDENTITY_2026_10_24.md) confirms 14,632 fewer
  allocation requests in the 8,192-element QuickSort probe, preserving snapshots
  and storage backing. The unique JIT case uses 25% fewer requests. 1,092 library
  and 27 targeted integration tests pass; this candidate's release gates remain
  open. Argument vectors remain the larger source of requested bytes.

- **Discarded native function locals (`93e7caa`).**
  [Allocation evidence](LOCAL_RETURN_2026_10_24.md) records 53% fewer requests
  and 68% fewer requested bytes for the unique 8,192-element JIT QuickSort
  diagnostic, with backing/snapshot behavior preserved. Successful VM-managed
  native returns avoid constructing dead local maps; standalone, loop and
  error synchronization remain intact. 1,090 library and 38 targeted integration
  tests pass. Frozen release checks pass all 150 test-size cases; QuickSort JIT
  shows a 1.1631x median paired local speedup (6/7 favorable), while VM is flat.
  Two peak-memory samples per variant do not show a JIT RSS reduction. Host
  speed limits changed; production, competitive and platform gates remain open.

- **Immutable dispatch facts (`16b96d5`).** [Diagnostic evidence](CALL_METADATA_2026_10_24.md)
  reproduces repeated instruction scans on native callbacks and verifies lazy,
  copy-on-write-invalidated metadata caching. 1,089 library tests and 29 targeted
  integrations pass. The debug-library probe loses its body-length-dependent
  dispatch cost. Frozen release verification now passes all 150 test-size
  checksums; seven-round QuickSort timing remains inconclusive under changing
  CPU limits. See `call-metadata-release-2026-10-24.json`; this acceptance does
  not extend to later runtime edits or production/platform gates.

- **Bounded native callback root reuse (`09c8944`).** Inactive root stores retain
  capacity across calls while active recursive calls remain isolated. The
  [root-pool report](ROOT_POOL_2026_10_24.md) records a 34.6% reduction in JIT
  allocation requests and 41.3% reduction in requested bytes for the diagnostic
  8,192-element QuickSort kernel, with unchanged copy-on-write behavior. This is
  a partial call-frame improvement; native calls still pass through VM machinery.
  Release measurements and checksum gates are recorded separately from these
  structural allocation results. Production and competitive gates remain open.

- **Resumed on 2026-10-24 at the user's request, starting with QuickSort.**
  `9b5694f` restores hot-call promotion for helpers reached from native code;
  `50f8357` transfers callback arguments without repeated vector growth/reversal.
  All 1,086 library tests (8 ignored) and 19 targeted integrations pass, including
  VM/interpreter/JIT snapshots, argument ordering and error recovery. Release
  measurements are recorded in [the QuickSort call-cost report](QUICKSORT_CALL_COSTS_2026_10_24.md).
  All 150 frozen-release test-size checksums pass; timing gains remain
  inconclusive under throttling. Production/competitive gates remain open.

- [Latest five-variant comparison](COMPARISON_LATEST_2026_10_24.md) reruns the
  original baseline, latest VM/JIT, Goose and Rust in seven rotating rounds:
  140/140 checksum checks pass across four selected cases. Paired gains are
  2.17x for VM JSON generation and 2.63x for JIT QuickSort versus baseline VM.
  JIT QuickSort still takes approximately 153x Goose's and 175x Rust's time.
  That measurement-only task did not resume implementation or establish an
  all-50 competitive score. The two patches were applied after this comparison,
  in the resumed work above.

- **Historical pause checkpoint on 2026-10-24 (now resumed).** The
  [root-cache release matrix](checksums-2026-10-24-native-root-cache-release.json)
  verifies all 150 unchanged test-size checksums on frozen `faf9f95`, with
  source/input/binary identities and 100 measurement reports verified. The
  [partial production record](production-paused-2026-10-24.json) retains six
  completed JIT timeouts on `3482e53`; Matmul T8 was cancelled at the user's
  request, not classified as a failure. The production sweep is stopped.
  Callback hot-call promotion and argument-transfer fixes were unapplied drafts
  at that checkpoint; they are now applied in the resumed work above.
  Production completion, competitive speed and platform gates remain open.

- [Recursive-call production sample](native-recursive-call-profile-2026-10-24.json)
  retains matched executable/map evidence from frozen `3482e53` running
  Brainfuck::Recursion. Allocation/copying and environment lookup appear in the
  sample; mapped callers identify `jit_call_function` debug-flag checks as an
  investigation target. This is not a completed production result or a timing
  comparison, and any change must preserve observable environment behavior.

- [Validated root-slot read hints](native-root-read-cache-2026-10-24.json)
  eliminate repeated handle-map lookups without retaining array owners or raw
  element pointers. Every hit validates the live slot and handle. Compiler
  admission excludes single-read nonloop locals; repeated sites and loop reads
  can reuse hints. Seven rotating helper-probe rounds favor warm hints, while
  forced misses add overhead. These debug-library diagnostics do not establish
  release, production or competitive speed; those gates remain open.

- [Positional-argument release matrix](checksums-2026-10-24-native-positional-release.json)
  verifies all 150 unchanged test-size checksums on frozen `fc48aec`, including
  positional callback ownership transfer. All 100 measurement reports and
  source/input/binary identities are verified. Production and competitive
  acceptance remain open; this predates the root-slot cache candidate.

- [Scalar-array production outcome](production-matmul-scalar-array-2026-10-24.json)
  records the unchanged `3482e53` Matmul timeout after two warmups and four
  measured iterations. No final checksum was obtained.
- [Packed-scalar prototype](packed-scalar-prototype-2026-10-24.json) compares
  136-byte generic elements with explicit 16-byte payload/kind records. It is
  exploratory code, not a runtime feature. Packing has an upfront cost and can
  regress cold or single-pass reads; safe reuse and end-to-end measurements are
  required before admission. Concurrent host work limits timing conclusions.

- [Post-scalar-read production profile](native-array-read-profile-2026-10-24.json)
  on frozen `3482e53` still identifies indexed-read lookup, bounds and tag decoding
  as hot work. Matching-map disassembly uses a 136-byte generic array-element
  stride. This motivates measured investigation of guarded native array/loop
  access; sample counts do not prove cache misses or predict speed. The run was
  still active when recorded, so production checksum acceptance remains open.

- [Positional native arguments](native-positional-arguments-2026-10-24.json)
  reduce warmed callback allocations from 5/8/12 to 3 at arities 1/4/8, and
  requested bytes from 1,580/2,236/3,540 to 916 per call. Compiler-proven SSA
  initialization permits moving the incoming vector; other functions retain
  named binding. All 1,082 library tests (8 ignored) and 152 integrations pass,
  including named fallback, value semantics and allocation bounds. No elapsed
  speed or current-candidate release/production acceptance is claimed.

- [Frozen scalar-array release matrix](checksums-2026-10-24-native-scalar-array-release.json)
  verifies all 150 unchanged test-size checksums across JIT, VM and interpreter
  on `3482e53`. Binary, source, input and report identities are retained, with
  100 measurement reports verified. This predates positional argument transfer;
  production and competitive acceptance remain open.

- [Checked native array reads](native-scalar-array-reads-2026-10-24.json)
  avoid scalar materialization and reuse validated lookups for owned elements.
  All 1,080 library tests (8 ignored) and 151 integrations pass, including
  three-runtime value semantics and exact scalar-bit/error checks. Seven paired
  debug-library probe samples show median reductions of about 44% for scalar
  arrays and 9% for owned arrays; these are helper costs, not suite speedups.
  Current-candidate release checksums and production acceptance remain open.

- [Frozen production Matmul outcome](production-matmul-local-borrow-2026-10-24.json)
  on `d777339` reached warmup completion but hit the unchanged 600-second process
  deadline during measured work. Its checksum remains unverified. This diagnostic
  run predates the scalar-read candidate and does not establish a speed comparison.

- [Production scalar-read profile](native-scalar-index-profile-2026-10-24.json)
  on frozen `d777339` identifies indexed-read dispatch, value copying and scalar
  materialization as hot paths. The sample was taken during the first unchanged
  production Matmul warmup; it does not establish a checksum pass or speedup.
  The checked array-read change above addresses this helper cost; a new release
  profile is still required to assess the remaining end-to-end costs.

- [Native call setup reservation](native-call-setup-reservation-2026-10-24.json)
  reduces warmed allocations per call from 6/10/15 to 5/8/12 at arities 1/4/8
  by reserving parameter-map capacity and borrowing cached function names.
  1,079 library tests (8 ignored) and 151 integration tests pass. This preserves
  the named-parameter path and value semantics; it is not a competitive timing
  result. The positional transfer evidence above addresses eligible owned-argument calls.

- [Current QuickSort CPU profile](native-call-boundary-profile-2026-10-24.json)
  preserves the unchanged checksum on `d777339`. Allocation, copying, native
  call dispatch and environment lookup are visible costs. Matching executable
  UUID/map hashes support attribution; counts include idle threads and are not
  speed ratios. A targeted regression confirms the compiled-call bridge already
  consumes unique collection storage and preserves shared snapshots; the suspected
  retained-argument copy was disproven. [Warmed call setup measurements](native-call-setup-allocations-2026-10-24.json)
  record 6/10/15 allocations per call at arities 1/4/8, excluding compilation and
  argument construction. Reducing named parameter binding and dispatch setup is
  the next measured candidate; not every measured allocation is assumed removable.

- [Current frozen release matrix](checksums-2026-10-24-native-local-borrow-release.json)
  verifies all 150 unchanged test-size checksums on `d777339`, including fixed
  dictionary COW and borrowed local indexed reads. All source/input/binary hashes
  are pinned; JIT and VM measurement reports pass hash and schema verification.
  Fallback remains allowed. Production and competitive acceptance remain open.

- [Borrowed local indexed reads](native-local-indexed-borrow-2026-10-24.json)
  eliminate per-read temporary root creation for live SSA locals: 9/129 reads
  previously issued 11/131 handles and now issue at most 2. Named/captured loads
  retain their existing paths. 1,078 library tests (8 ignored), 150 integration
  tests and formatting pass, including three-runtime snapshots and error parity.
  This is deterministic ownership-cost evidence, not an elapsed-time speedup.

- [Fixed dictionary COW](fixed-dictionary-cow-2026-10-24.json) removes container
  copies from compiled local field reads: all six width/read-count combinations
  now allocate zero bytes, previously up to 17,965,056 bytes. Snapshot writes
  detach shared storage; unique writes reuse it. Shared-DAG depth and cached-child
  traversal are covered. 1,077 library tests (8 ignored), 184 integration tests,
  formatting and minimal-feature compilation pass. Current-candidate release
  checksums, production sizes and competitive performance remain unverified.

- [Mapped production CPU profile](native-indexed-read-profile-2026-10-24.json)
  retains the unchanged Matmul::Single production timeout on `7468173`: no
  first warmup completion within 600 seconds. Matching binary UUID and linker-map
  hashes support symbol attribution. Operand duplication, root bookkeeping and
  memory copying are visible costs; sample counts include idle threads and are
  not native-time percentages or competitive timing. Direct borrowing of live
  SSA locals for indexed reads is the next measured candidate.

- [Mapped borrowed-read release](checksums-2026-10-24-native-borrowed-read-release.json)
  passes 150 unchanged test-size checksums on `7468173`. Disk exhaustion
  interrupted the interpreter driver after 35 recorded passes; the remaining
  15 cases were rerun and the complete matrix verified with recovery provenance.
  This predates fixed-dictionary COW work and does not close production gates.

- [Borrowed native reads](native-borrowed-read-allocations-2026-10-24.json) avoid
  cloning the entire source container for indexed and field reads. Reading the
  first field 128 times from fixed dictionaries of 8, 128 and 1,024 fields now
  allocates zero bytes in the helper probe. Root lifetimes, returned snapshots
  and errors pass 1,073 library tests (8 ignored) and 166 integration tests
  across VM, interpreter and native execution. This is not competitive timing.

- [Frozen pop/index release JIT checksums](checksums-2026-10-24-native-pop-index-release-jit.json)
  and the [VM](checksums-2026-10-24-native-pop-index-release-vm.json) and
  [interpreter](checksums-2026-10-24-native-pop-index-release-interpreter.json) reports
  verify all 150 unchanged test-size cases on `70c7beb`, including report hashes
  and schemas. This predates the scalar-depth optimization; production and
  competitive acceptance remain open.

- [Scalar depth allocation checks](scalar-depth-allocations-2026-10-24.json)
  reduce preallocated native scalar appends from one allocation per element to
  zero at 8, 128 and 1,024 elements. Shared scalar depth checks bypass traversal
  allocation; container checks retain their depth policy. The change passes
  1,072 library tests (8 ignored) and 56 targeted integration tests, including
  both-runtime snapshot semantics. This is allocation evidence, not speed acceptance.

- [Archived production checkpoints](superseded-production-checkpoints-2026-10-24.json)
  retain 17 verified successful reports from superseded candidates. The earlier
  JIT matrix had 11 passes and 39 timeouts; its VM run stopped after 6 passes
  and 10 timeouts. The next candidate recorded five JIT timeouts before
  cancellation. Active cancelled cases remain unverified. These loaded-host
  runs do not establish competitive speed or current-candidate acceptance.

- [Native indexed replacement](native-indexed-replacement-2026-10-24.json) and
  [guarded pop/remove](native-pop-rebinding-2026-10-24.json) pass 1,071 library
  tests and 184 integration tests, with no exclusions. Unique storage reuse,
  retained snapshots, fallback effects and readonly errors have coverage.
  [Readonly initialization](native-readonly-locals-2026-10-24.json) is repaired.
  The [current static audit](post-pop-index-native-surfaces-2026-10-24.json)
  reports 158/165 bodies with no unsupported opcode; remaining first blockers are
  await (1), closure creation (3), captured loads (2), and try (1). This is not
  native time coverage or speed. Final-candidate all-50 release checksums,
  production checks, profiling and controlled competitive acceptance remain open.


- [Native checked nested writes](native-nested-writes-2026-10-24.json) pass
  1,061 library tests and 155 integration tests, including both-runtime parity.
  Unique rows remain reusable; retained snapshots force COW. Captured named loop
  destinations remain on the VM to preserve frame-cell resolution. The
  [static audit](post-nested-write-native-surfaces-2026-10-24.json) reports 139 of
  165 bodies without an unsupported opcode, up from 126; this is not native time
  coverage. Nested append, pop/rebinding, await, captures/closures and exception
  surfaces remain. Production and controlled competitive acceptance are open.

- [Checked owned global assignment](native-owned-global-assignment-2026-10-24.json)
  repairs constructor-produced values silently shadowing globals in private
  native locals and bypassing readonly errors. The earlier iteration release
  build was cancelled after these defects were reproduced; it produced no new
  frozen release or all-50 acceptance result.

- The [guarded-string static audit](post-string-guard-native-surfaces-2026-10-23.json)
  reports 89 of the same 165 function bodies without an unsupported opcode,
  up from 83. Guarded array/byte append and iteration are now first blockers in
  25 bodies each. This is admission evidence only; current-source checksums,
  actual native time coverage and comparative performance remain separate gates.

- The [frozen field-read candidate](checksums-2026-10-23-native-field-jit.json)
  passes all 50 JIT test-size checksums. It still has the independently reproduced
  native-loop method receiver defect repaired at `ca251a7`; the checksum result
  does not override that defect or certify the repaired source. Production,
  native time coverage and comparative speed gates remain open.

- The [post-field static audit](post-field-native-surfaces-2026-10-23.json)
  reports 83 of the same 165 function bodies without an unsupported opcode,
  up from 72. Remaining first blockers include iteration (25), guarded append
  (23), nested binding updates (9), guarded string append (8) and pop/rebind (6).
  These counts are static function surfaces, not native time coverage or speed.
  The field candidate still requires its all-50 and production gates.

- The [native-struct candidate JIT matrix](checksums-2026-10-23-native-struct-jit.json)
  passes all 50 unchanged test-size checksums with binary/input identities,
  raw measurement hashes and all 50 schemas verified. It includes native struct
  construction and owned-loop persistence, but predates the callback field fix
  at `66b60a8`. Forty cases record native function entries; ten record none:
  Binarytrees::Arena, Matmul::Single/T4/T8/T16, Json::Generate, Etc::Sieve,
  Etc::NeuralNet, Etc::LogParser and CLBG::Nbody. Entries do not measure native
  time coverage. VM fallback is allowed; current-HEAD, production and competitive
  speed acceptance remain open. The separate native-array debug interpreter
  matrix completed with 49 passes and a retained MergeSort deadline failure;
  see [debug interpreter results](debug-interpreter-deadlines-2026-10-23.json).

- The [post-constructor static audit](post-constructor-native-surfaces-2026-10-23.json)
  scans the same 165 port function bodies. 72 report no unsupported opcode, up
  from 39 at the owned-entry stage. Remaining first blockers include guarded
  append (23), field reads (22) and `ForNext` (20). These are function-level
  static results, not native execution, benchmark coverage or speed evidence.
  Later unsupported instructions and runtime admission guards still require work.

- The [owned-argument release candidate](owned-argument-entry-2026-10-23.json)
  committed at `f559179` passes all 50 unchanged test-size checksums in each of
  JIT, VM and interpreter modes (150/150). Runtime source and executable hashes,
  inputs and raw report schemas are pinned and verified. Fallback remains allowed;
  this does not establish full native coverage, production acceptance or competitive
  speed. Subsequent native stack-index and field-error fixes remain separate
  candidates requiring their own gates. The earlier debug interpreter QuickSort
  and MergeSort deadline failures remain recorded in the
  [completed debug interpreter matrices](debug-interpreter-deadlines-2026-10-23.json);
  the separate release pass does not erase them.

- The [literal-enabled current matrix](checksums-2026-10-23-native-literals-jit.json)
  at `70606e8` records 42 passing test-size JIT checksums and eight compression
  runtime failures. All report hashes and schemas were verified. Newly compiled
  callers expose the incomplete cold callback dispatcher, which rejects
  `MakeArray(0)` in the shared vector helper. This candidate is not accepted;
  repair cold-call dispatch and re-run the unchanged matrix before widening
  owned argument entry. The frozen release build of this revision has the same
  known source gap and must not be promoted based on its successful build alone.
  The subsequent [cold callback repair](cold-callback-2026-10-23.json) checks
  unsupported helper bodies before execution and routes them through the full VM
  bridge while preserving cached native calls. Its independent regression covers
  native entry, array construction, prior effects, error recovery and unchanged
  assignment targets. All eight compression cases now pass with reports verified;
  the [repaired all-50 JIT matrix](checksums-2026-10-23-cold-callback-jit.json)
  at `8a593f9` now passes, with every raw report hash and schema verified.
  This debug run allowed VM fallback and overlapped other local work. It does
  not certify the newer owned-argument candidate, production sizes or speed.
  Separate VM/interpreter verification remains in progress.

- The [tagged-owned candidate](checksums-2026-10-23-owned-values-jit.json) at
  `e2797da` passed all 50 test-size JIT checksums, with frozen input/binary identity,
  raw report hashes and measurement schemas verified. 34 cases recorded native
  entries; fallback remained allowed. This debug diagnostic run predates operand
  cleanup, overlapped local tests/builds and provides no competitive timing evidence.
  Its separate VM/interpreter runs are not included in that result.

- The JSON serialization change has a measured paired median improvement of
  2.916x on this laptop. That is one workload, not a suite-wide speedup.
- Nested assignment now avoids reproduced Arc allocation churn. Its noisy paired
  timings do not establish an end-to-end speedup.
- The earlier verified test-size
  [interpreter](checksums-2026-10-23-guarded-append-release-interpreter.json),
  [VM matrix](checksums-2026-10-23-guarded-append-release-vm.json) and
  [JIT matrix](checksums-2026-10-23-guarded-append-release-jit.json) passed all 50 at
  `ac3fe6c` in release builds, with hashes/schemas verified and fallback allowed.
  This frozen candidate predates native-loop callback resumption and retains
  the operator gaps recorded below. It cannot certify current HEAD.
  No complete current production-size run exists.
- The frozen `ac3fe6c` release production run recorded a
  [Binarytrees::Obj timeout](production-binarytrees-timeout-2026-10-23.json)
  at the ten-minute process deadline, after six warmups and progress through
  measured iteration six of thirty. This is incomplete checksum coverage.
  The shared-host run is diagnostic; a stripped-symbol CPU sample cannot
  identify the responsible source hotspot. Other production cases continue.
- All 50 test-size cases passed with JIT enabled at `33a85f9` using a ten-minute
  per-case debug deadline. VM fallback was allowed. This establishes neither full
  native coverage nor speed acceptance. The earlier `c4a07cd` run and its MergeSort
  timeout/retry remain in the evidence history.
- Experimental JIT immediate argument guards, callback errors, whole-function
  integer global assignments and Boolean return/callback tags have regression
  coverage. Native scalar operand type guards also have regression coverage.
  Native short-circuit joins now carry operand payloads and type tags; regressions
  cover native execution, mixed stack joins and direct recursion. Cold callbacks
  now restore recursion depth after return. Boolean/integer named global loads and
  whole-function stores preserve scalar types. Float/null immediate values now
  have native payloads, float arithmetic/comparisons and typed loop-local
  persistence. Function arguments now carry immediate kinds through cached calls,
  callbacks and direct recursion. String/object values, collection helper boundaries
  and full loop/script binding parity remain open. Checked native
  integer arithmetic now has overflow/divisor guards and regression coverage.
  General-value compilation and owned-value native entry remain unfinished.
- Shared-laptop contention and throttling prevent reliable idle-machine score
  predictions. A clean measurement environment is an acceptance requirement.
- The [native census](NATIVE_EXECUTION_CENSUS.md) at `0c77115` passed all 50
  test-size checksums with validated measurement reports. 34 cases recorded native
  function entries and 16 did not; this is not full native coverage. A QuickSort
  CPU sample identifies VM dispatch, opcode cloning and call-frame setup costs.
- The [native lifetime-hook run](checksums-2026-10-23-native-root-lifetimes-jit.json)
  at `7f89e13` passed all 50 test-size checksums. Binary/config/port/runner hashes,
  all raw report hashes and all measurement schemas were verified. Four native
  entry/dispatch counters match the earlier census for every case. This debug
  run allowed fallback and overlapped local builds; it supplies no speed evidence.
  It predates the `0f4934f` lazy-scope guard and does not close the eight owned-value
  failures, current-HEAD verification or production gates.

## Non-negotiable rules

Keep algorithms, inputs, iterations, warmups, RNG behavior and checksums unchanged.
No benchmark-name special cases, precomputed answers, omitted work, or native
kernels written just for a benchmark. General-purpose optimizations must have
independent semantic and cost tests. Preserve value semantics, alias isolation,
binding rules, errors, cancellation and capability enforcement.

Keep VM, interpreter and JIT evidence separate. A fallback may preserve correctness
but is not evidence of faster compiled execution. Timeouts and crashes stay visible.
No Kostya contact or upstream submission is authorized by this plan.

## 1. Establish a trustworthy baseline

- Pin source commits, executable hashes, compiler versions, build flags and the
  unchanged upstream suite revision for Kujo, Goose and Rust.
- Complete the current candidate's 50 test-size cases in both runtimes before
  interpreting its timings. Record explicit JIT results separately.
- Run all 50 production cases with recorded deadlines and memory limits. Save
  partial results; distinguish wrong result, crash, timeout and successful run.
- Use a quiet, adequately cooled host. Run samples sequentially, without our own
  build or profiling jobs. Record power, throttling and competing load.
- Collect at least seven valid samples per comparison, alternate baseline and
  candidate order, and retain raw measurements. Separate startup, preparation,
  warmup, measured work, total elapsed time and peak resident memory.

Gate: reproducible reports for every case, with missing results explicitly marked.
No estimated idle-host timings qualify as a baseline or an acceptance result.

## 2. Find where time and allocations actually go

- Profile representative numeric loops, recursion, sorting, collections, strings,
  parsing, serialization, graphs and parallel work. Extend to every outlier.
- Attribute CPU time to actual execution paths; measure allocation count/bytes,
  copying, reference-count traffic and peak memory separately.
- Measure successful native compilations, native entries, fallback reasons and
  coverage by function/loop. Compilation attempts alone do not prove JIT coverage.
- Add isolated reproducers and input-size scaling measurements for each suspected
  cost. Distinguish linear work from accidental quadratic copying.

Gate: every optimization has an observed hotspot, a failing semantic or cost
reproducer where applicable, and an unchanged end-to-end workload to validate it.
Prioritize by measured potential benefit and implementation risk.

## 3. Finish the compiled-execution correctness foundation

Work in this order, with a separate small commit for each coherent change:

1. Fix JIT global stores through checked binding resolution. Cover mutable globals,
   local shadowing, readonly globals, changing value types and nested callbacks.
2. Preserve result types across native boundaries, including Boolean versus integer
   results. Audit all supported argument and return representations.
3. Add checked native arithmetic and catchable errors: overflow, division/modulo
   by zero and signed boundary cases. Preserve effects before the failure, prevent
   effects after it, and never replay failed work.
4. Audit every native helper's status/error path, stack/frame state and callback
   boundary. Verify locks, captures, recursion and imports under repeated calls.
5. Add differential generated programs comparing values, types, output and errors
   across interpreter, VM and JIT, including failures after hot-loop compilation.

Gate: new regressions fail on the baseline and pass on the fix; existing relevant
contracts and the library suite pass. JIT stays opt-in until broader acceptance.

The [dictionary constructor audit](native-dictionary-constructors-2026-10-23.json)
adds native dynamic-key and unique fixed-key construction with tagged operands,
retained key-vector lifetime and VM nesting/error checks. Repeated constant keys
now preserve source-order evaluation and last-value-wins semantics in the VM;
that form still falls back because its string constants are not native operands.
Numeric literal-key coercion remains a separate VM/interpreter compatibility gap
(recorded in the audit); no suite speedup or production acceptance is claimed.

The [owned operator audit](native-owned-operators-2026-10-23.json) fixes native
string arithmetic/ordering and preserves struct operator dispatch and errors
through the reference VM. Immediate numeric paths remain native. Constants remain
gated until retained literal lifetimes and specialized local-helper behavior are
verified; bounded root storage is not evidence of reduced string copying or speed.

### Remaining native ownership gate

The [current owned-argument probe](owned-argument-profile-2026-10-23.json) at
`70606e8` confirms the entry exclusion independently of literal support: 200 calls
to the same identity function yield 101 native entries with integer arguments,
but zero with array arguments (both outputs pass). Preserve frame-local ownership,
cache transitions, recursion, errors and alias isolation when widening this gate.
A five-second debug NeuralNet sample places 975 of 3,420 execution-thread samples
inside checked nested assignment. Copy-on-write appears in that path, but its
necessity is unproven; require unique/aliased allocation reproducers and release
profiling before changing it. Neither observation supplies a speedup estimate.

The [native literal audit](native-literals-2026-10-23.json) adds string and byte
constant lowering through compiler-retained values and the existing owned-value
codec. Literal loads share immutable backing buffers and release temporary roots;
tests exercise source-bytecode disposal, repeated compilation retention, native
functions and specialized loops. A focused string-return probe moves from zero
to 101 native entries while preserving output. General owned argument entry and
aggregate constants remain gated. This is coverage/storage evidence, not a
competitive speed result; older constant exclusions below describe prior audits.

The [operator-loop audit](operator-loop-audit-2026-10-23.json) reproduces two open
gaps at `ac3fe6c`: constant appends bypass struct `op_add`, and specialized native
loops deadlock when an operator callback reacquires the globals mutex held by the
loop invocation. The latter is confirmed by native-entry logs and a sampled lock
stack; VM/interpreter pass the same probe. Preserve checked callback dispatch and
resume state without replaying effects before widening native coverage. These
probes are independent of the all-50 matrix. The
[loop callback resume fix](native-loop-resume-2026-10-23.json) repairs the deadlock
by persisting the full operand stack and locals, exiting native execution and
resuming the exact VM instruction after releasing the lock. Calls and struct
operators may therefore fall back; immediate numeric operations stay native.
The original deadlock probe and regression checks now pass with native entry
verified. The [constant append operator fix](constant-append-operators-2026-10-23.json)
also restores `op_add` dispatch, snapshot isolation and immediate result tags in
native functions; locked loops resume before invoking the operator. The separately
observed interpreter/VM discrepancy in assigned operator-error results is repaired
by preserving pending error values from unary and binary operator methods instead
of replacing them with Null. The regression preserves assignment targets and
prior effects, checks repeated failure/success recovery in all three modes, and
checks an operator failure after verified native-loop entry. See the
[operator error audit](operator-error-parity-2026-10-23.json). This closes that
specific parity gap, not the broader native ownership or production gates.

The [fused character-append audit](guarded-append-fusion-2026-10-23.json) adds
an ordinary bytecode fallback to the VM fast path.
It guards independent mutable bindings and integer/string types, preserves counters
when the loop is skipped, and processes at most 4,096 appends per dispatch before
checking cancellation again. Native eligibility no longer advertises this opcode
without a lowering implementation. Native fusion and release speed measurements
remain open; the guarded VM path is not a substitute for those gates.

The [specialized-loop audit](native-loop-collections-2026-10-23.json) replaces
untyped integer-dictionary access with guarded dense integer access and tagged VM
helpers. Generated code uses an explicit data/length ABI, checks key/value types,
bounds and binding mutability, and refreshes views after helper writes. Loop global
stores use checked binding resolution under the existing environment lock. Native
entry regressions cover representation changes, missing keys and readonly failures;
prior side effects persist exactly once. This does not establish current all-50
production correctness, elapsed speed or general owned native-entry coverage.

[Whole-function indexed reads](native-indexed-reads-2026-10-23.json) now use callee bindings and tagged VM indexing
semantics rather than caller-slot integer-dictionary helpers. The regression
covers typed array/dictionary results, strings, bytes, negative/fractional indices,
snapshot isolation and catchable errors. This does not widen owned argument entry
or validate specialized integer-dictionary loop paths.

The [indexed-write audit](native-indexed-writes-2026-10-23.json) now covers native
whole-function writes through callee SSA/named bindings, mutable/let/const checks,
owned values, alias isolation and catchable bounds/byte errors. It shares the VM's
existing value update implementation, mechanically checked against the original.
Unique array storage survives 1,000 writes; 8/128 compiled writes retain bounded
root storage. Specialized loop helpers and remaining representation costs still
need separate audits; no elapsed-time improvement is claimed.

The [constant append audit](native-constant-appends-2026-10-23.json) repairs callee
binding selection, retains embedded suffix literals, preserves COW snapshots and
persists owned loop locals with their type tags. Native helper failures stop later
effects; VM append type errors are catchable. Unique preallocated storage is reused
for 1,000 appends, and compiled 8/128-append cases retain at most four root slots of
capacity. This is correctness/storage evidence, not an elapsed-time improvement.
Fused append-until and specialized indexed collection helpers remain unaudited;
general string/byte constants and wider owned native entry remain gated.

Converted operand paths now release Pop, overwritten SSA locals, VM-marshaled
callable/arguments and consumed equality/logical operands. Dup and SSA local loads
receive distinct roots. Compiled discard/overwrite/callback tests keep root-slot
capacity at most four at both 8 and 128 repetitions. This is deterministic storage
evidence, not a suite speedup or proof of every opcode's lifetime behavior.
`ReleaseDeadLocal` now has native lowering, caller-slot isolation, Null readback
and copied-parameter compilation regressions. The bounded-storage test also covers
dead-local release while a separate operand copy remains live. Legacy collection
helpers, loop-entry/local persistence and wider owned-entry coverage still require
their own checks before the gate closes.

The latest [borrowed-equality audit](jit-owned-equality-2026-10-23.json) passes all
11 focused probes in interpreter, VM and JIT modes with native entries verified.
Owned comparisons now use `Value::equals` instead of handle identity. This closes
the expanded equality reproductions below, but does not establish general native
coverage, dead-operand lifetime bounds, production correctness or speed acceptance.

Earlier [named-operand audit](jit-owned-loads-2026-10-23.json): ordinary named loads,
callable marshaling, collection callback arguments and named assignments now use
tagged owned values. All eight original probes agree across the three modes.
Three added equal-value probes still fail: comparing handles happens to pass the
original unequal-string check but breaks identical strings and structurally equal
arrays/dictionaries. Equality remains open, alongside dead-operand lifetime work,
legacy specialized collection paths and broader native coverage. Older audit
counts below describe their recorded candidates, not this latest state.

The [owned-value lifetime foundation](../../docs/NATIVE_OWNERSHIP.md) now provides
recyclable roots, frame-local cleanup and explicit return promotion. Native
boundary/function lifetime hooks are connected, with lazy storage for immediate
paths. Generated owned operations still need integration; mechanism and ABI-probe
tests are not a language-level parity pass. Tagged VM-stack conversion now uses
the codec for callback results and argument marshaling. The refreshed
[owned-stack audit](jit-owned-stack-2026-10-23.json) repairs the native callback
return probe; seven original mismatches remain. Dead operand and overwritten-local
release is still required before claiming bounded storage for long owned loops.

The [owned-value audit](jit-owned-values-2026-10-23.json) reproduces eight native
mismatches on `0c77115`, with VM/interpreter agreement and measured native entries
for every probe: string/array/dictionary returns, native callback returns and
arguments, bytecode callback arguments, string equality and empty-string truth.
The [reproducer](probes/native_owned_values.kujo) records failures rather than
treating its own successful exit as a parity pass. These are current correctness
gaps in that baseline, even though the earlier all-50 benchmark checksum run passed
with fallback. The refreshed audit above tracks the one repaired case separately.

The string-global audit still returns an integer marker after compilation. Fix the
complete owned-value path before widening owned argument guards or claiming typed
native parity: named loads, callable values, locals, operand-stack joins, operators,
callback arguments/results, assignments and returns must agree on representation.
Use the reference VM for operator/capability/error contracts that need a runtime
helper; retain native immediate operations where their guards prove the types.

Ownership storage must scale with live values, not loop iterations or the total
number of loads. Verify release at pops, overwritten/dead locals, normal returns
and every error exit, including recursion and nested callbacks. An append-only
handle vector lasting for a whole invocation is not an acceptable final design.
Keep alias snapshots and copy-on-write behavior intact across those boundaries.
The dead-local path now implements lifetime semantics and copied-parameter loops
have native compilation coverage; wider owned entry remains gated. Add bounded-live-storage and allocation/scaling
checks alongside both-runtime semantic tests before claiming a performance win.

## 4. Remove measured VM and representation costs

Investigate these as candidates, not as assumed defects:

- Call setup, argument preparation, local/global lookup and redundant validation.
- Collection ownership across calls/imports; field/index updates; append/pop/delete;
  nested rows and structs. Retain copy-on-write isolation for aliases and snapshots.
- Numeric dispatch and boxing in tight loops; byte access and string scanning.
- JSON parsing/mapping/serialization, temporary buffers and repeated conversions.
- Allocation lifetimes, reference counting and worker scheduling overhead.

Use borrowing, ownership transfer, reusable storage, specialized bytecode or compact
representations only where profiling and semantics justify them. Do not introduce
an arena, collector or broad representation rewrite without evidence that its
benefit exceeds its complexity and compatibility cost.

Gate: deterministic cost/scaling improvement plus unchanged semantics. Claim a
speedup only after paired release measurements; retain negative results too.

The [fixed-dictionary write allocation audit](fixed-dictionary-write-allocations-2026-10-23.json)
removes an unnecessary whole-value-vector clone for existing-key replacement.
Across 36 observations it saves one allocation per write; the 256-field/1,000-write
case saves 34,816,000 requested bytes on this build. Width-dependent write allocation
growth is eliminated while snapshots/nested aliases pass VM/interpreter/JIT checks.
Key lookup and snapshot creation costs remain; elapsed speed is not yet established.

## 5. Expand fast compiled execution

The [owned-entry surface audit](owned-entry-native-surfaces-2026-10-23.json)
records the first unsupported operation in 165 port functions. Eleven unchanged
test-size cases still record zero native function and boundary entries, despite
compilation attempts. Blockers include indexed reads, nested updates, field reads,
array/struct construction, iteration and rebinding operations. These static
findings identify work to investigate; they do not measure time spent per opcode.
Array-backed local loop admission is also separate work: the existing specialized
loop path admits unique integer maps, while ordinary indexed array locals are
rejected before compilation. Completing stack `IndexGet` alone does not close it.

The [nested-state allocation audit](nested-state-allocation-audit-2026-10-23.json)
finds no width-dependent repeated-copy cost in its direct/helper update fixture,
with or without a saved snapshot. All 96 observations preserve the expected result;
eight warmed allocation slopes match at widths 8 and 256. However, JIT-requested
cases record zero native entries: the helper first encounters unsupported
`IndexGet`, followed by `UpdateBindingPath`. Prioritize general native nested-path
reads/writes, with binding checks, error ordering and snapshot isolation, before
changing COW semantics for this pattern. These are allocation/coverage results,
not release timing or proof that all nested assignments are optimal.

- Add guarded typed paths for the numerics and collections found in real profiles,
  including floats where workloads require them.
- Reduce VM/native crossings, redundant loads and boxing on proven hot paths.
- Extend calls, recursion, imports and captures with correct ownership and errors.
- Evaluate inlining and loop optimizations after type guards and exit state work.
- Measure compilation/startup costs and native coverage. Consider caching or AOT
  only if those costs or platform constraints justify the additional system.

Gate: hot paths demonstrably execute native code, pass differential tests and show
repeatable end-to-end improvement. Correct fallback must resume from valid state
without duplicating side effects. Never remove safety checks merely to win a score.

## 6. Verify all 50 production workloads

For each candidate milestone, update the existing 50-row checklist with source and
binary identity, mode, checksum, sample count, elapsed time, peak memory, comparator
ratios and unresolved limitations. Re-run all test-size cases in VM and interpreter;
complete production VM coverage and explicit interpreter/JIT coverage separately.

Gate: 50/50 unchanged production checksums in each mode claimed as verified, no
unclassified failure, and no substitution of historical evidence for current code.
Production interpreter timeouts remain incomplete coverage even if VM succeeds.

## 7. Enforce competitive and regression gates

For each case, divide Kujo's time by the faster measured Goose/Rust time. Track
milestones of at most 2x, then 1.25x, then below 1x with repeatable evidence.
Publish the entire distribution and worst cases; an aggregate cannot hide failures.

- Investigate repeatable slowdowns above 5% on controlled-host measurements.
  Treat noisy measurements as inconclusive, not as automatic passes or failures.
- Track startup and peak memory alongside execution time. Record and justify any
  memory tradeoff; do not exchange unbounded memory growth for lower elapsed time.
- Keep quick semantic/cost gates in ordinary CI and the full performance matrix on
  a controlled scheduled runner. Recheck apparent wins in independent sessions.

Gate: no unexplained correctness failure or repeatable regression; competitive
claims identify exactly which cases, modes, builds and hosts support them.

## 8. Release and integration acceptance

- Run formatting, build checks, library and relevant integration suites, language
  and CLI contracts, runtime parity and all-fifty checks on the final candidate.
- Verify release builds on supported operating systems and architectures. A macOS
  Intel result does not certify Linux, ARM or Windows behavior/performance.
- Validate local official-driver timing protocol and packaging before submission.
- Keep maintainer decisions separate: typed JSON mapping versus DOM conversion,
  indexed graph representations, worker-pool lifecycle and accepted build flags.
  Prepare concrete evidence and questions; obtain authorization before contact.

Gate: evidence attached to the release candidate, remaining limitations explicit,
and experimental features promoted only after their own reliability/platform gates.

## Work contract for each implementation batch

Reproduce → add regression → change the smallest responsible path → run both-runtime
and affected JIT checks → measure deterministic costs → build and measure release
candidates → update the checklist → commit and push a small reviewed change.

Batch expensive release builds across coherent tested fixes, but never label an
older binary as the current candidate. If a hypothesis fails, record the result and
choose the next measured hotspot. Finishing the plan means satisfying its gates,
not merely landing code in every area.

### Guarded collection native execution milestone

Native lowering now covers `TryAppendAssign` and conditional `ReleaseRebindLocal`.
The regression suite requires native functions and loops while preserving COW,
shadowed builtins, RHS destination replacement and immutable bindings. Deterministic
helper checks require unique array/byte allocation reuse at 8 and 128 updates,
retained-snapshot isolation and bounded root capacity. See
`native-guarded-collections-2026-10-23.json` for verification provenance.

The unchanged 165-function static audit now has 109 functions with no unsupported
opcode (previously 89). Remaining first blockers: ForNext 25, GuardAppendPath 8,
TryPopRebind 7, UpdateBindingPath 12, MakeClosure 2 and LoadCapture 2. This is
eligibility evidence only: actual native time coverage, final-candidate production
checksums, controlled competitive measurements and release/platform gates remain
open. Existing frozen benchmark runs do not certify this source revision.

### Native iteration and loop-return milestone

`ForNext` now has native normalized array/byte stepping and guarded generator
continuation handling. Reachable-edge stack analysis preserves early-return and
exhaustion control flow. Returns inside separately compiled loops resume the VM
at the return instruction for normal frame cleanup; regression coverage includes
the previously incorrect hot `while` return and owned return values.

The unchanged 165-function static audit now has 126 functions with no unsupported
opcode (previously 109). Remaining first blockers: GuardAppendPath 10,
TryPopRebind 7, UpdateBindingPath 16, MakeClosure 3, LoadCapture 2 and BeginTry 1.
This remains static eligibility, not native time coverage or speed acceptance.
See `native-iteration-2026-10-24.json` for verification and source provenance.
