# Code-owner reuse boundary: source and compile-only evidence

This narrows the next implementation mechanism; it is not a runtime change,
accepted reuse design, or speed result. The separate VM lifetime candidate still
needs its remaining platform artifacts and acceptance review. Source inspection
uses main `7eecf40b`; the compile-only checks use the same frozen baseline and
candidate libraries as the matched lifetime resource experiment.

## Verified trait boundary

Six compile-only checks pass their expected outcomes on the recorded macOS x64
toolchain:

| Type requirement | Baseline | Memory candidate |
| --- | --- | --- |
| `JitCompiler: Send` | Compiles | Compiles |
| `JitCompiler: Sync` | Rejected with E0277 | Rejected with E0277 |
| `Arc<Mutex<JitCompiler>>: Send + Sync` | Compiles | Compiles |

The Sync failures identify the backend's interior-mutability, raw-pointer and
symbol-lookup closure members. They are actual trait errors, not missing-library
or incompatible-metadata failures. The collector rejects unexpected error codes.
No binary is executed and no runtime unsafe trait implementation is introduced.

This establishes that exclusive movement between workers is expressible with
the existing Rust types. It does not establish pointer lifetime, correct reset,
cross-platform behavior, bounded retention, or profitable reuse. A mutex around
the compiler alone is insufficient: VM dispatch retains separate pointer maps.

## Source boundary that an implementation must preserve

| Owner/state | Current source and required treatment |
| --- | --- |
| Executable module and embedded constants | `JitCompiler` owns its JITModule plus literal values, dictionary keys, struct shapes, field/path names and append strings. Keep all these owners alive with every generated entry. |
| Whole-function dispatch | `VM.compiled_functions` and `compiled_fn_info` are keyed by immutable code identity. Transfer them with the compiler or invalidate them before compiler destruction; do not use the compiler's name-based inspection index as execution identity. |
| Loop dispatch | `LoopCacheKey` includes code identity, start/end and dense slots. Preserve the specialization key and guard invalidation; offset alone is insufficient. |
| Bytecode edits | `BytecodeChunk::deref_mut` assigns a new identity even for uniquely owned storage. A reused entry must miss after edits, and distinct same-named functions must remain distinct. |
| Cancellation | Compiler cancellation-check policy is immutable. Task VMs supply the current task token through each VMContext. A prior task token must never become part of a retained code owner. |
| Execution state | Globals, captures, capability policy, output, errors, stacks, roots, local slots, return values, dense views and VMContext pointers belong to the current invocation/VM. Never pool a live context with compiled code. |
| Admission/profiling | Function counts live in VM; loop admission/profiles and enabled state live in the compiler. Reuse must specify their reset/retention policy and preserve existing guard-reset behavior and debug-environment observation points. |
| Task routing | `tasks.rs` calls `VM::execute_language_task` inside `spawn_blocking`. That constructs a fresh policy-bound VM; async parallel-map wrappers submit another task. Pooling only the outer mapper does not reach the measured duplicated compilation. |

## Proposed first implementation boundary

Extract an exclusively owned bundle containing the compiler and all dispatch
metadata whose pointers depend on it. Preserve per-task VM construction and its
policy/environment/output installation. A bounded pool would lease a whole code
owner, remove it under a short mutex hold, and release the mutex before execution.
Return an owner only after all synchronous native frames have unwound and no
VM-local pointer cache refers to it. On panic or uncertain state, discard through
the verified owner destruction path instead of recycling an uncertain owner.

This is an exclusive-lease proposal, not simultaneous execution from one compiler.
Holding the pool lock across native execution or callbacks risks deadlock and
serializes workers. A cache miss must permit progress without waiting for an
owner held by a parent awaiting a nested task. Do not add `unsafe impl Sync`,
share bare entries, or reuse mutable VM/environment state to bypass this boundary.

Scope the pool to an explicit execution lineage with cancellation-policy keys;
do not default to process-global retention or rely on Tokio assigning successive
tasks to the same thread. Propagation through nested task/callback routes must be
proved before claiming coverage of the measured async path. Owner-count limits
alone are insufficient: each compiler can accumulate code and embedded metadata.
Define and measure byte/code-entry budgets and whole-owner eviction before
acceptance; the current statistics do not establish a retained-byte limit.

## Required implementation tests and acceptance observations

- Repeated task jobs reuse native functions and loops; retain exact outputs and
  prove reduced compile attempts in the existing 64/40/120-cell diagnostics.
- Same name/different bytecode, mutated bytecode, changed argument kinds and
  dense-slot layouts cannot select stale code or skip guard invalidation.
- Sequential and overlapping jobs receive their own globals, captures, output,
  capabilities and cancellation tokens. Cancel one, then successfully run another.
- Nested tasks/callbacks, errors and panic do not deadlock, lose admission permits,
  recycle active code, or carry stale errors/roots into the next task.
- Returned closures/generators/snapshots survive pool eviction and parent VM
  destruction; public standalone compiler lifetime remains unchanged.
- Pool-disabled, no-JIT and interpreter controls preserve existing behavior.
- Retained code/metadata stay within explicit budgets over many distinct chunks,
  task lineages and eviction cycles. Windows reclamation remains a separate gate.
- Run all-runtime regressions, all150 test-size checks, targeted production and
  platform gates, then paired unchanged-workload measurements. Compilation stays
  inside normal execution/timing; no benchmark-name special cases.

These are requirements for the next candidate, not completed test claims. This
first exclusive-reuse experiment would not close broader concurrent code sharing,
native collection access, call marshalling, or all-50 production/competition work.

## Reproduction and retained evidence

`probes/collect-jit-owner-traits.rb BASELINE_RLIB CANDIDATE_RLIB DEPENDENCY_DIRECTORY NEW_OUTPUT`
compiles `probes/jit_owner_traits.rs` with metadata-only output. The receipt records
the toolchain, exact argv, library/source hashes, expected failures and diagnostics.
`jit-owner-traits-2026-11-03.json.gz` retains those receipts, scripts and inspected
source snapshots. Libraries are identified by hash rather than embedded. Rebuild
and freeze matching libraries/dependencies if the original local artifacts expire.
