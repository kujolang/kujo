# Runtime concurrency completion record

Started 2026-09-26 from merged wave-1 main
`87fae36dd331b185d29256f74d2a92f1aefc5ea5` on
`runtime/generator-async-spawn-completion`.

The owner requested implementation of the remaining concurrency, interpreter,
reliability and provider/trust work after authorizing the wave-1 main merge.
This record tracks implementation; it is not a completion claim.

## Implementation decisions

The Phase-A recommendations D1–D6 are the implementation defaults for this work:

- D1: generator aliases share one instance. Concurrent/reentrant resume rejects;
  completed generators remain completed.
- D2: return completes without yielding. The first generator failure propagates;
  later resumes report the cached failure. Caller restoration is unconditional.
- D3: async calls submit eagerly to bounded execution resources and return a
  promise. Body failures belong to that promise. Independent task ordering is
  unspecified; awaiting establishes the dependency ordering.
- D4: spawn snapshots referenced transferable bindings, preserving mutability.
  Unsupported referenced values reject before scheduling. Ordinary lexical
  siblings do not acquire shared-state semantics. Resource handles require an
  explicit supported transfer contract.
- D5: detached spawn remains no-value/no-join, with no implicit process-exit join.
  Detached failures require bounded reporting on stderr, never unsolicited stdout
  or execution from evidence. Exact diagnostic limits are specified with the
  implementation and protected by CLI tests.
- D6: spawn_task executes a zero-argument callable or reports arity mismatch;
  completion is cached for repeated waiters. Cancellation and timeout remain
  distinct from proof that external effects stopped.

These decisions intentionally change the characterized broken behavior and must
be accompanied by changelog, language/runtime documentation and regression tests.
No new syntax, async generators, struct generator methods, tracing collector or
replacement executor is implied. Wave-1 snapshot capture identity stays fixed.

## Work inventory

1. Owned generator continuation, full existing opcode dispatch, restoration,
   identity, terminal errors, lazy consumption and supported nested control flow.
2. Atomic promise completion and multi-waiter behavior; async execution, mutation
   ownership, bounded resources and authority propagation.
3. Real task/spawn execution, explicit transfer, observable detached errors,
   cancellation/completion and channel deadlock/parity fixes.
4. Returned recursive interpreter closure and namespace import limitations.
5. Reproduce and remove LSP/docgen test variability without weakening guards.
6. Review provider authentication, adapter attestation/materialization,
   preservation and uncertain-effect recovery with offline negative tests;
   validate live providers when concrete test targets are available.
7. Cross-feature contracts, performance, canonical docs and full release gates.

Provider trust is not removable by changing an actor label or accepting an
idempotency string. Unknown effects must remain fail-closed. Live provider
certification, actual suspension support and external sink guarantees require
provider-specific evidence; these will not be inferred from offline fixtures.

## Slice 1: VM generator continuation

The VM now resumes its ordinary dispatcher, with owned operand/frame/handler and
local-scope continuation state. A generator holds only a weak reference to its
runtime globals; cross-environment migration rejects rather than silently using
another runtime's globals. Creation/resume capability policies intersect. Locks
are released while executing; reentry and nested resume depth are bounded. Root
return/error releases continuation resources. Yield preserves existing expression
stack behavior, including yielded null. No new unsafe code is introduced.

`ForNext` performs one lazy step and the compiler removes iteration stack residue.
Arrays still use direct indexing without a per-item option allocation. Scope and
handler restoration is shared with ordinary execution rather than a partial opcode
list. Existing capture cell identity, binding metadata and lexical self survive.

Checks so far: `cargo check`; generator continuation 10/10; closure capture 21 + 9
passing (the existing interpreter recursion probe remains ignored); parity 115/115.
The baseline characterization/callback/capture suites passed before source edits.
The updated characterization suite (13/13) and imported callbacks (9/9) also pass.
Interpreter generator and other concurrency work remain in progress.

## Slice 2: interpreter generator continuation

Interpreter generators now own shared continuation state instead of copying a
statement index on each alias. Explicit block, loop, for and handler frames retain
nested statement progress; return completes, errors are cached, and scope/capability
state is restored on each resume. Captured lexical scopes are retained while root
globals stay live in the owning interpreter. Generator handles do not copy root
globals into their continuation. Different runtime ownership rejects explicitly.

The initial targeted run passed 14 generator contracts and 13 characterization
cases. Characterization assertions now require the repaired generator behavior in
both runtimes. Collection callbacks containing `ForNext` use full VM dispatch.
Nested yield expressions still require continuation work; this slice does not
claim they are implemented by the interpreter. Async, spawn and recursive
interpreter closures remain under implementation.

## Slice 3: shared promise completion (validation in progress)

Every promise receiver now anchors one shared future. Await, timeout, aggregation,
parallel-map and cooperative VM polling obtain independent waiters. A waiter no
longer replaces the producer channel with a closed dummy receiver. Completed
results and producer-drop failures remain available to every alias; dropping or
timing out a waiter leaves the original producer available. Existing value/cache
fields remain as compatibility metadata; the producer result has one owner.

Promise validation: `cargo check` passed. The compiled
`promise_completion_contracts` test executable passed all 6 cases, including
pending multi-waiter wakeups, repeatable rejection/producer drop, cooperative
polling, timeout recovery, and duplicate handles in `promise_all`. A preceding
combined cargo invocation stopped at the new channel backpressure test: it
exposed an existing VM channel-method duplicate-receiver arity mismatch. That
separate channel fix is under test; it is not a promise failure.
