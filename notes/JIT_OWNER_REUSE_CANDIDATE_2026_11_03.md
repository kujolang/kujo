# Exclusive code-owner reuse candidate

The initial ownership-bundle commit now has a verified seven-job Linux gate;
see `benchmarks/langarena/JIT_OWNER_LINUX_GATE_2026_11_03.md` for its exact
1,360-pass scope. Later candidate revisions still require their own gates.

Base: `cd5f2b20` on `codex/native-performance-campaign-20261102`.
Candidate branch: `codex/jit-owner-reuse-20261103`.

This candidate is not integrated or accepted. It begins the separately gated
reuse implementation described in
`benchmarks/langarena/JIT_OWNER_REUSE_BOUNDARY_2026_11_03.md`.

The first patch places the private compiler, function/direct-argument entry
maps, native name metadata and inline call cache in one exclusively owned
`VmCodeOwner`. The inline cache also contains a native function pointer and must
not outlive that compiler. Reclamation moves from VM destruction to owner
destruction; the standalone public compiler contract stays unchanged.

Invocation state and function admission counters stay in the VM. No pool,
task-route change, concurrent sharing, new unsafe trait implementation, or speed
claim is included. A regression moves the owner to a fresh VM on another worker,
drops the original VM first, requires the same compiled entry, executes it and
checks returned values after final owner destruction. Existing lifetime, code
identity, callback and generator tests remain applicable.

Additional boundary regressions execute code against different VM globals,
retain embedded string literals through both VM destructions, and toggle the
previous/current cancellation tokens independently. They preserve the compiler's
cancellation-check policy while requiring each invocation's current token. The
targeted platform workflow also runs the owner boundary without JIT on Linux.
These checks are pending execution, not proof that task pooling is safe.

The next patch records per-compiler compilation attempts, successful definitions,
completed finalizations and emitted machine-code bytes across offset/loop,
function, direct-argument and script compilation. A loop cache hit must not grow
these counters; changed bytecode must. Rejected compilation increases attempts
without claiming successfully finalized code. Definition bytes are recorded
before context clearing/finalization; unknown buffer size saturates accounting
instead of appearing free. These counters are **not total retained memory**:
backend code-page rounding, GOT/PLT, compiler metadata and embedded owners need
separate bounds/resource evidence before pool acceptance. No pool is enabled.

The workflow offers a Linux-only owner/accounting gate to validate the next
candidate without repeating all platform builds for every preparatory patch.
Final acceptance still requires the applicable full platform refresh.

Visible metadata now has a separate checked quota weight covering table capacity,
loop slot vectors, profiling tables, function names, dictionary keys, shapes,
field/path names, append strings, literal keys/payloads and exposed backend
declaration names/signatures. Shared payloads may be charged more than once.
Unknown literal kinds reject accounting. A regression covers large string/byte
literals, deduplicated retention, spare table capacity and an unknown payload.
This is a logical retention quota, **not a measured total-memory bound**;
backend-private capacities, relocations, symbol/GOT/PLT storage, allocator/control
bytes and page rounding remain outside it. Reset/drop of transient compiler
context and the separate physical-resource gate remain required before accepting
pooling. The method is staged and not yet connected to task execution.

Verification is pending. Run the release library suite, no-JIT coverage and
callback/task/generator contracts before accepting this boundary. The subsequent
pool requires cancellation-policy matching, lineage propagation, admission/reset
policy, explicit code/metadata budgets, whole-owner eviction and panic/reentry
coverage. Those requirements remain open; this patch alone does not reduce
compilation counts.

Build/test gates should run on CI while the frozen local Brainfuck production
sequence continues. Do not substitute this candidate for its frozen executable
or change the canonical port during that sequence.

## First Linux result and feature-guard correction

Run `41627350511`, job `125887926893`, artifact `13226839913`, exact
`cb931728f8cf7d59856f84830b4213a87eec2146`: all six optimized owner-lifetime
tests passed, including cross-worker transfer, fresh globals/literal ownership,
current cancellation and executable-entry revocation. Exact names, source and
summary were independently checked. This is not whole-candidate acceptance.

The same job failed the no-default-features library build with 18 diagnostics;
no no-JIT tests ran, and the integration step was skipped. Five network-only
tests and their helpers referenced `network_policy` or HTTP functions compiled
out by that feature configuration. The four affected source files are byte-for-
byte unchanged from accepted base `cd5f2b20`; the owner extraction did not create
those references. The raw logs and identities, including the failed outcome,
are retained in `benchmarks/langarena/jit-owner-linux-boundary-cb931728-2026-11-03.json.gz`.

The follow-up applies `runtime-network` guards only to those network tests,
helpers and imports. It adds an explicit disabled-network diagnostic test and
keeps the no-default-features gate unchanged. Networking-enabled tests remain
enabled. The correction and new test still need execution; it does not turn the
earlier failed job into a pass.

The workflow now collects default-feature accounting and integration results
after a successful owner gate even when the independent no-JIT gate fails.
Cancellation still stops those steps, and a failed no-JIT step still fails the
job. This prevents an unrelated feature-configuration failure from hiding later
default-feature test outcomes; it does not relax acceptance.

## Idle compiler reset prerequisite

The candidate now has an explicit idle-only reset boundary. It rejects changed
cancellation policy, disabled compilers, unfinished/rejected compilation counts
and legacy offset-only entries. Successful reset replaces transient IR context,
admission/retry/blocked-site tables and type profiles, releasing their spare
capacity. Code-identity loop caches, machine code and embedded literal/key/shape
owners remain together. Pointer-keyed append/key caches retain the Arc allocation
used as their key, so dropping the source chunk does not recycle that key while
its compiled owner survives.

The loop regression now requires fresh admission after reset, the same cached
native pointer and unchanged compilation/byte counts. A separate rejection test
covers policy, disabled state, attempt/definition imbalance and offset entries.
The targeted workflow and exact-membership verifier include the new test.
Execution remains pending CI; formatting and static inventory checks are local.
This method is not connected to task execution. Pool quotas, owner eviction,
route propagation, panic/reentry and physical-resource gates remain required.

## Bounded idle storage prerequisite

A staged private VmCodePool now moves whole owners through a short mutex hold.
Cancellation policy selects an idle owner; misses return immediately rather than
waiting for an owner held by a parent task. Explicit constructor limits cover
idle-owner count, native definitions, emitted code and logical metadata weight.
Return first applies idle reset, removes invocation-side name/inline caches and
charges retained VM dispatch-map capacity in addition to compiler metadata.
Empty, incompatible, over-budget or poisoned-lock returns discard their owners.
Rejected-owner destruction occurs outside the pool lock. These quotas do not
claim to bound opaque backend allocations or process RSS.

The new owner test crosses a worker thread, checks exclusive checkout and native
pointer identity, executes the retained function in a new VM, checks all four
quota rejection paths and policy mismatch, poisons storage and verifies fail-
closed behavior, and retains the returned value after eviction. Runtime execution
is pending CI. No task route uses this pool yet; production defaults, lineage
propagation, panic-safe return and whole-candidate resource/performance gates are
still open. No new unsafe code or unsafe trait implementation is introduced.

## Initial task-route integration (unaccepted)

Interpreter state now lazily holds a lineage-local pool only when JIT is
requested. Candidate defaults retain at most two idle owners, each capped at
512 native definitions, 4 MiB emitted code and 8 MiB visible metadata. These are
provisional logical ceilings, not measured physical memory bounds. Ordinary and
detached task submission carry the pool through both bytecode and AST routes;
child task interpreters inherit it. VM task execution leases a cancellation-
compatible owner into a fresh VM and returns it only after synchronous execution
returns. Rust panic unwinding skips return and destroys the VM-owned compiler.
Normal language errors retain no invocation state in the returned owner.

A regression runs identical loop bytecode with two different task globals,
requires correct distinct results and unchanged nonzero compilation/byte counts,
and checks that a non-JIT interpreter does not allocate a pool. Existing task,
cancellation, capability, snapshot, callback and generator gates remain required.
The reproduction baseline is the existing 64/40/120 compilation-economics
controls; this candidate has not yet run that comparison or its new test.

This is partial route integration: reusable callback VM bridges and parallel-map
worker construction still require explicit lineage propagation and tests. No
whole-candidate acceptance, all-150 refresh, physical-resource result or elapsed
speed claim exists. Changes remain isolated on the candidate branch. Local builds
remain deferred while the frozen production sequence runs; CI executes runtime
checks. Production files and accepted main source are unchanged.

## Callback and parallel-worker lineage propagation

Parallel bytecode mapper VMs now inherit the submitting interpreter's pool;
inner async tasks therefore reuse the same lineage instead of starting a pool
per outer wrapper. Interpreter-to-bytecode callback bridges and operation-owned
synchronous callback VMs likewise inherit the caller pool. Outer wrappers keep
their own code owners; this change targets the reproduced inner-task duplication.

A new end-to-end task-policy test compares one versus four sequential jobs through
parallel_map and regex callback routes. It checks exact outputs in VM/interpreter/
JIT, requires native participation only in JIT, and requires nonzero compile
entries to remain flat as job count grows. It is pending CI execution, not a
measured performance result. The artifact verifier derives integration count from
exact pinned-source membership rather than hardcoding sixty, allowing the new
61st test without relaxing name/summary checks; historical verifier controls pass.

Remaining acceptance includes current-source runtime tests, overlapping task and
error/panic/cancellation reuse controls, bounded retention under diverse code,
physical resources, full economics diagnostics, all150, production and platforms.
The older feature/accounting gate41630118425 now reports terminal success; its
raw artifact still needs independent verification. It does not certify these new
route changes. No current candidate is merged into accepted main.

The corrected feature/accounting gate is now independently verified: see
`JIT_OWNER_ACCOUNTING_GATE_2026_11_03.md` for 71 targeted passes on exact
`fc56848e`, raw artifact provenance and six verifier controls. This supersedes
pending-execution statements for that revision's three accounting tests and two
minimal-feature tests only. Later reset/pool/task-route checks remain pending.

## Task reuse recovery regression

A new task-execution regression warms and retains an owner, cancels the previous
invocation's token, then runs with a fresh token. It separately cancels the current
task and requires a subsequent successful invocation, triggers an indexed-access
error, replaces the function with a different same-named bytecode function, and
returns to the original. Retained array results must remain correct after pool
destruction. This exercises task-level code selection and invocation isolation;
it does not replace overlapping-task, panic-path or capability tests. Execution
is pending CI. The pinned artifact verifier includes its exact owner-test name.

Reset revision06e896cf run41632651515 now reports owner/no-JIT/accounting steps
successful, with integration still running. These API step statuses are not yet
independently verified terminal artifacts. Later pool/route gates remain pending.

The earlier `cb931728` Windows boundary is independently verified at 65 targeted
passes; see `JIT_OWNER_WINDOWS_BOUNDARY_2026_11_03.md`. Windows reclamation remains
open, and this earlier revision does not certify later pool/task-route changes.

## Capability transition regression

The task reuse corpus now alternates filesystem-read permission allowed/denied/
allowed against a fixed temporary fixture and the same bytecode. It requires the
successful fixture contents, the specific capability-denied error, one retained
owner and unchanged nonzero compilation counts across all three invocations.
Thus fresh policy must be honored with actually retained code, not by avoiding
reuse. The fixture is test-owned and removed automatically. Exact owner membership
is included in the artifact verifier; runtime execution remains pending CI.

Pool revision0a0dd452 now reports its owner-test step successful; minimal-feature
checks are running. Current-source acceptance still requires completed raw
artifacts, broader concurrency/recovery/resource gates and measured economics.

## Overlapping task regression

An end-to-end regression now holds two workers concurrently at channel gates after
each has performed a hot loop and private array mutation. Both must announce
readiness before either is released. A third task runs after they return, while
first/second results and the original input remain observable. VM/interpreter/JIT
must produce the exact independent arrays; JIT must report native participation,
and non-JIT must not. The existing thirty-second process deadline catches hangs.
This controls overlap explicitly rather than assuming worker scheduling produced
it. Runtime execution remains pending; it does not by itself establish resource
bounds or a compilation-speed gain. Pinned integration membership automatically
includes the new 62nd integration test.

Reset prerequisite06e896cf is now independently verified at72 targeted Linux
passes; see `JIT_OWNER_RESET_GATE_2026_11_03.md` and its raw evidence archive.
This supersedes pending reset-execution statements only, not later pool/task-route
acceptance. The initial task-integration run41633864624 reports owner-step success
but is still running its remaining steps; raw terminal verification is pending.

## Worker unwind control

A focused Rust-worker regression now checks out a compiled owner into a fresh VM,
executes it, then deliberately panics after native frames have returned but before
normal owner return. It checks the exact panic payload, so an unrelated assertion
failure cannot satisfy the control. The pool must stay empty and unpoisoned, a
fresh VM must compile/execute successfully afterward, and earlier returned arrays
must survive final destruction. This tests the Rust unwind boundary; it does not
claim support for unwinding across an active native ABI frame. Runtime execution
is pending CI, and physical reclamation remains a separate gate.

## Async economics acceptance wiring

The Linux targeted workflow now runs the unchanged forty-cell async economics
collector after successful integrations. A separate `reuse` verification mode
requires one compilation for direct/serial jobs, nonzero bounded compilations
for concurrent jobs, unchanged native participation and exact task/output/input
coverage. Baseline mode preserves the original one-compilation-per-job oracle.
Loop-only misses/hits must agree with compilation/reuse counts. Concurrent cells
report their observations without assuming a scheduling-dependent speed gain.

The collector now freezes task and callback route sources as well. The verifier
requires the source manifest and still checks raw outputs, measurement hashes,
argv/configuration, statuses and all forty exact cells. Six synthetic parser
controls pass: baseline/reuse accepted; duplicate cell, failed outcome, serial
recompilation and lost native execution rejected. Synthetic controls are not
benchmark results. Runtime diagnostic collection is pending CI. Instrumented
wall spans are not an uninstrumented speed comparison or self CPU attribution.

Fresh-checkout correction: the first source-manifest control rerun failed because
`port.kujo` is generated and absent from the candidate checkout. CI now runs the
canonical Kujo builder before collection, and synthetic verifier tests create
self-contained source fixtures in a temporary working directory. All six controls
pass after this correction. The earlier failed local check is not counted as a
pass; no runtime diagnostic execution has completed yet.

## Distinct-code eviction control

The task-owner corpus now executes32 distinct same-named hot-loop functions with
a one-idle-owner/two-definition quota. It checks exact version-specific results,
retained definition/code/visible-metadata bounds, exactly ten whole-owner evictions
(one new definition per task), and all returned arrays after final pool drop.
The exact eviction count rejects a candidate that merely discards every owner
because reset or accounting failed. Execution is pending CI. This is a logical
quota and ownership control, not a physical heap/RSS reclamation measurement.

Bounded storage0a0dd452 now has73 independently verified targeted Linux passes;
macOS Intel cb931728 has66. See `JIT_OWNER_POOL_GATE_2026_11_03.md` for exact
identities and limits. These close their prerequisite artifact reviews only;
later actual-task/recovery/economics and physical-resource acceptance stay open.

## Isolated Linux resource diagnostic

A dedicated ignored test (explicit CI invocation only) now runs the actual task
VM path with/without a pool, for same/distinct bytecode at32/1024 tasks. Each of
eight cases gets its own process and180-second deadline. It warms one compiler,
then records process RSS and total executable virtual mappings every32 tasks and
after pool destruction. Native/compile counters require the intended execution:
pooled same-code compiles once; unpooled/distinct code compiles once per task.

The collector pins the exact test executable SHA, source commit, argv/environment,
exit/signal and raw log hashes. The verifier checks eight exact cases, every
checkpoint and all counters, leaving resource observations explicitly review-
pending. Observations include common harness/runtime allocations (including the
retained receipt vector); they are not total live heap or pool-attributable bytes.
No physical-memory limit or speed gain is asserted before paired data review.
The Linux workflow runs this only after successful integrations. Local syntax,
formatting and inventory checks pass; actual resource execution remains pending.

## Verified task and route prerequisites

Initial task integration ff3409ec, bridge integration 5ae0d6b4 and overlapping-task
revision a7dd8001 now have independently verified terminal Linux artifacts: 74,
75 and 78 targeted passes respectively. The last includes fresh capabilities,
cancellation/error recovery and rebinding tests. See
`benchmarks/langarena/JIT_OWNER_TASK_ROUTE_GATE_2026_11_03.md` for exact provenance
and raw archive. Eighteen verifier controls pass across the three artifacts.
These supersede earlier pending statements only for those revisions. Later
unwind/eviction, economics, resources and final combined acceptance remain open;
no elapsed-speed gain is established.

## Resource verifier negative controls

Sixteen self-contained synthetic controls now test the resource verifier without
building or executing Kujo. The valid eight-case receipt is accepted explicitly
as review-pending. Missing/duplicate cases, timeout, changed logs, wrong environment
or invocation, missing checkpoint, repeated compilation, lost native execution,
invalid retained-owner state, invalid memory values, duplicate receipt, wrong
source and changed binary are rejected without writing an acceptance receipt.
These controls pass locally and are wired before future CI resource collection;
results are uploaded alongside diagnostics. They validate the evidence parser,
not resource behavior or an optimization speedup. Existing live runs predate this
workflow addition and are not restarted.

## Numeric guard and bytecode-edit reuse regression

A new task-path regression alternates integer/float arguments through one retained
owner and compares exact numeric kinds/results with unpooled VM execution. It then
edits a constant in cloned bytecode, requires a new code identity, and alternates
edited/original calls to reject stale dispatch while preserving the original.
The pool must retain an owner with emitted native code. This is targeted coverage
of an explicit boundary requirement, not a proof of every specialization or dense
slot layout. Source-pinned artifact membership includes the new test. Runtime
execution remains pending CI; no production runtime implementation changed.

## First verified compilation economics result

Source d9803cb6 now passes all forty async economics cells in CI and independent
bounded archive review: direct/one-worker serial cases compile once for16/64 jobs,
with unchanged outputs, task counts and native participation. Concurrent cases
also reduce compilation events in this observation; scheduling affects counts.
This is not elapsed-speed evidence. The binary was rehashed by CI but not uploaded,
so archive verification explicitly preserves its identity as a CI receipt. Source,
generated port, raw outputs, metrics and invocations are independently checked.
Six synthetic and five archive parser controls pass. See
`benchmarks/langarena/JIT_OWNER_REUSE_ECONOMICS_2026_11_03.md` for counts and limits.
Unwind1910a363 and economics-source d9803cb6 each have79 independently verified
targeted Linux passes. Later resources, final platform/all150 and elapsed gates
remain open; no candidate integration follows yet.
