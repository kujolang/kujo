# Generator, async and spawn Phase-B handoff

Phase-B implementation follow-up: [runtime completion record](RUNTIME_CONCURRENCY_COMPLETION.md).
This document preserves the pre-implementation audit and accepted design decisions;
its old broken-behavior descriptions are historical, not the current runtime contract.

Historical evidence dates: 2026-09-25–26, before Phase B began. The following
starting-point instructions describe that handoff, not the current branch state.

## Authoritative starting point

The integration baseline is `dace9c1837ca7c7b27c406ea9b8944cc6aa7f0f3` on
`integration/runtime-hardening-wave-1`. Start a **new** branch named
`runtime/generator-async-spawn-completion` from the verified integration tip
(or the main commit that contains it after an authorized merge). Do not resume
implementation on `runtime/generator-async-spawn-audit`. Main was not changed.
The integration review records final verification and the following documentation and test
commit. Do not substitute the original Agent-1 tip: integration fixes retain
function-local imports and scoped constants in escaped closures.

## Closure architecture to preserve

The authoritative contract is v1 per-closure snapshots, **not** shared
parent/sibling bindings. A new closure copies selected values into owned
`Arc<Mutex<Value>>` cells. Aliases and calls of that closure share those cells;
separate closures and nested creations snapshot again. Collections retain their
existing value/COW behavior. Binding kinds travel with captures.

- `BytecodeChunk.capture_sources: Option<Vec<CaptureSource>>` pairs with
  `upvalues` in index order. Sources are `Local(slot)`, `Upvalue(index)`,
  `Environment(name, kind)` and `Named(name)`. `None` is the retained legacy
  manually constructed chunk path, not the compiler's normal output.
- `Compiler::resolve_capture` and `finish_child` forward references through
  intermediate functions. Local lexical slots outrank outer captures.
  Selected imports register named bindings. Import-all exports use lazy named
  capture requests at the importing ancestor, with global fallback for absent
  names. Only requested values are retained; no whole-environment VM capture.
- `LoadCapture` / `StoreCapture` use `VM::capture_slot`, a lazy indexed view of
  the existing captured map. Stores enforce `BytecodeBindingKind`; ordinary
  local values remain plain slots. Name lookup/locking exists on capture creation,
  first access and compatibility paths, not on every ordinary local access.
- There are no stack-backed open captures to promote or close. Frame/scope
  teardown drops owners; escaped closures own their snapshots. The legacy
  VM-wide `Upvalue` opcodes are not this mechanism.
- `CallFrame.captured_slots` is a derived cache. Restored generator frames rebuild
  it lazily from saved captured maps and kinds. A continuation must retain Arc
  identity, local initialization/kind vectors, caller chunks, handlers and policy
  without copying the underlying capture values into fresh cells on each resume.
- `lexical_self` installs a named recursive function in its active frame, outside
  its capture environment. Keep that cycle-avoidance property.
- The existing imported-callable bridge shares capture cells/globals and carries
  capability policy and recursion limits. Closure-bearing/captured/self-bound
  functions bypass incompatible name-only JIT caches. VM generator capture access
  works in the bounded existing dispatcher; general continuation does not.

## Updated dependencies

| Phase-B item | Classification | Current fact / required work |
|---|---|---|
| Generator locals across yield | UNCHANGED | Local slots already survive covered straight-line yields; full scopes/handlers and lazy iteration remain. |
| Generator closure capture | SIMPLIFIED BY AGENT 1 | Explicit descriptors and owned cells exist; reuse them when adding MakeClosure to complete resume dispatch. Preserve snapshot semantics. |
| Generator frame restoration | CHANGED BY AGENT 1 | Rebuild the derived captured_slots view; preserve saved cell Arcs and lexical self, not an imagined open-upvalue list. |
| Async closure capture | SIMPLIFIED BY AGENT 1 | VM lexical identity is explicit and tested. Interpreter environment ownership and overlapping invocation behavior still need design. |
| Async mutable capture | CHANGED BY AGENT 1 | Same-closure aliases share state; siblings do not. Arc/Mutex on a value does not make a read-modify-write transaction atomic. |
| Spawn lexical capture | NEW DEPENDENCY DISCOVERED | Its separate compiler still omits ordinary capture setup and discards the body. Adopt the descriptor/import rules only with an approved transfer contract. |
| Spawn shared state | UNCHANGED | Transferable snapshots and explicit shared_* resources remain distinct; no implicit sharing of lexical siblings. |
| Cross-runtime callbacks | UNCHANGED | Keep policy, arity, globals, errors and the 32-entry synchronous bridge guard; task recursion needs a separate bound. |
| Frame lifetime assumptions | CHANGED BY AGENT 1 | Owned snapshot cells outlive frames; no retained parent frame or stack pointer is required. |
| Open-cell promotion/closing redesign | NO LONGER NEEDED | Not part of the chosen v1 snapshot architecture. |
| Async ordering and detached error/shutdown policy | BLOCKED | D3/D5 (and the other D1–D6 compatibility decisions) remain proposals requiring owner resolution before dependent implementation. |

Old assumption → new fact: “Agent 1 supplies shared sibling cells” → independent
snapshots; “grandparent capture requires promotion” → descriptor forwarding and
new snapshots; “frame movement requires closing open captures” → retain owned
Arcs and rebuild caches; “upvalue landing pending” → use this verified integrated
baseline; “ordinary spawn compilation inherits closure resolution” → it still does
not. Agent 3 changes schemas and type-inference depth accounting, not execution
frames, scheduling or capability policy.

## Implementation order

1. Resolve D1–D6 from the completion plan and record compatibility/error/shutdown
   decisions. Preserve snapshot identity; do not reopen that settled contract.
2. Generator owned continuation, complete dispatch and unconditional caller
   restoration. Then identity/reentry/terminal state, handlers, scope ownership
   and lazy iteration. Remove first-yield loop exhaustion only with bounded lazy
   consumption. Do not introduce async-generator or struct-generator-method support.
3. Atomic promise terminal state and waiter registration, including producer
   failure, timeout/reawait and concurrent consumption.
4. Language async scheduling/context parity and same-closure mutation ownership,
   using the existing executor and capability restrictions.
5. Explicit transfer validation and real native task callable execution; then VM
   spawn execution with bounded tasks, detached errors and shutdown semantics.
6. Supported cross-feature tests, performance measurements, public docs and the
   full release gate. Keep each slice reviewable and independently validated.

Likely files: `src/vm.rs`, `src/compiler.rs`, `src/bytecode.rs`,
`src/interpreter/{mod,value,environment,async_runtime}.rs`,
`src/interpreter/native_functions/async_ops.rs`, targeted tests and canonical docs.
CLI changes require machine-readable contract updates. No new syntax, package
system, policy engine or replacement executor is implied.

## Conflict-sensitive surfaces and required tests

Do not replace whole compiler/VM files during merges. Preserve capture descriptor
ordering, local slot/kind/initialization metadata, import fallback, self-binding,
`CallFrameData`, `VmExecutionSnapshot`, callback normalization and JIT guards.
Preserve Agent 3's `TypeChecker::infer_expr` balanced depth guard. Workflow metadata
is evidence, never permission to bypass ordinary Kujo capability restrictions.

Keep these green:

- `cargo test --test closure_capture_audit --test closure_capture_contracts`
- `cargo test --test imported_vm_callback` (including function-local import regression)
- `cargo test --test vm_interpreter_parity_surfaces`
- `cargo test --test runtime_semantics_characterization`
- `cargo test --test native_api_security_boundaries --test database_lifecycle`
- `cargo test --test workflow_control_contracts`; run its two ignored artifact
  tests explicitly with fresh Dispatch fixture paths as documented in the review.
- `cargo test --lib type_checker::tests` and canonical docs/CLI contracts.
- `cargo run -- test --runtime vm`, `cargo run -- test --runtime dual`, strict
  Clippy and `bash scripts/release_gate.sh --full`.

Specific cross-feature guards include
`vm_generator_retains_snapshot_across_yields_after_factory_return`,
`vm_and_interpreter_match_async_named_nested_capture_mutation`,
`vm_and_interpreter_match_async_named_nested_capture_isolation`,
`imported_async_function_preserves_callback_globals`, and
`imported_function_can_await_vm_callback_results`. Preserve the two actual-artifact
schema tests, `actual_cross_component_failure_evidence_conforms` and
`actual_workspace_reexecution_artifacts_match_shared_schemas`, alongside the
effect/error/re-execution negative contracts. The old spawn counter probe does
not establish execution; `vm_spawn_body_is_discarded` records the real baseline.

Characterization assertions of broken behavior must change alongside each
intentional fix, with a supported oracle; do not remove them just to get green.
The existing ignored interpreter returned-recursive-closure probe is not a pass.
Current generator, async and spawn limitations remain those in the corrected
completion plan. Arbitrary cyclic values still lack a tracing collector.

## Definition of done

All approved G/A/S/X cases in the completion plan pass in their supported runtimes;
continuations restore callers on errors; identities and snapshot mutability remain
correct; promises support repeat/concurrent consumers; tasks execute under bounded
resources and inherited authority; unsupported transfer/semantics reject explicitly.
No unexplained startup/local/call/closure or control-journal regression, no weakened
security or error contract, and all required gates pass with skips and external
limits individually disclosed. Workflow retry never becomes an async scheduler or
an authority grant. Release documentation must describe observed behavior only.
