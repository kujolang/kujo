# Generator, async, and spawn completion: Phase A

Audit date: 2026-09-25. Source baseline: `719d6f7` (Kujo 1.5.0 working
source). Branch: `runtime/generator-async-spawn-audit`.

**Scope: audit, characterization, and implementation planning only.** No runtime,
compiler, syntax, closure, or frame implementation changes are included. Do not
start Phase B on the old audit branch. The integration addendum below updates
the original audit; source anchors and the baseline validation ledger remain
historical evidence for 719d6f7.

## Wave-1 integration addendum (2026-09-26)

The [Phase-B handoff](GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md) supplies the exact
verified base and current capture API. Start `runtime/generator-async-spawn-completion`
from that integration baseline, or main after it contains the integration. Phase A
is complete; Phase B has not begun. The original audit below is historical where
it describes pre-integration compiler resolution. Its generator/async/spawn gaps
remain current unless explicitly updated here.

Agent 1 preserves per-closure snapshots. New siblings and descendants copy values;
aliases and calls of one closure share owned cells. There are no open stack captures
to promote or close. `CallFrame.captured_slots` is a derived lazy view of saved
capture maps and binding kinds. The VM generator dispatcher now handles indexed
capture loads/stores, but still lacks general continuation and closure creation.
Integration also restores function-local import captures and scoped constants.
Agent 3 adds workflow schemas and balanced type-inference depth accounting; it does
not change frame ownership, capability policy or task execution.

D1–D6 remain proposed decisions. Shared sibling binding semantics are not one of
those open decisions and must not be introduced under async/spawn completion.
The updated dependency table and G11/G12/A06/X03 oracles below supersede earlier
shared-cell or open-upvalue recommendations.

## Executive summary

Straight-line generator iteration, sequential promise reawait, native async I/O,
cooperative VM await, and several async capture/callback paths already work.
The remaining work is more specific than the historical roadmap implies:

- VM generator resume is a second, partial opcode interpreter. It copies caller
  and generator stacks/frames, omits complete handler/caller execution context, cannot
  call functions, and exhausts on the first yield if the chunk contains any loop
  backedge. Normal VM execution already has a richer suspension snapshot.
- Interpreter generator progress is a top-level statement index copied with the
  value; only its environment is shared. Nested continuation, alias identity,
  return-versus-yield, and errors need explicit contracts.
- VM language async calls execute in the calling VM before wrapping a return in
  a promise. Interpreter language async calls submit interpreter execution to a
  persistent Tokio executor immediately. Pending native promises can suspend the
  cooperative VM; legacy/blocking VM execution and interpreter await block.
- VM `spawn` builds and discards a closure. Interpreter `spawn` detaches an OS
  thread with a filtered binding snapshot. Native `spawn_task` is a third path:
  it accepts AST functions, sleeps 1 ms, and returns null without running them.
- Existing spawn parity evidence only asserts a shared counter is nonnegative;
  it cannot distinguish successful execution from no execution. Existing
  generator creation tests do not prove Fibonacci or nested resumability.

Ready state: **NEEDS ARCHITECTURAL DECISION**. The evidence and ordered work plan
are ready; identity/error compatibility, async start ordering, and detached
spawn error/transfer policy need the small decisions listed below. The integrated closure prerequisite is documented in the handoff;
its snapshot contract remains binding on Phase B. No request to implement these
decisions is implied by completing Phase A.

## Evidence and reading ledger

Source anchors below use symbols and baseline line numbers; re-find symbols
rather than applying these offsets after rebase. `C` means source-confirmed,
`T` means exercised by the new characterization suite, `E` means existing
coverage, and `U` means a precisely scoped unverified edge. Source deductions
are not represented as executed tests. No network research is needed to describe
this repository; no external concurrency performance numbers are asserted.

| Material inspected | Consequence for this plan |
| --- | --- |
| `AGENTS.md`, `README.md`, `ROADMAP.md`, `CHANGELOG.md` | VM default, v1.5 scope, no broad implementation; current DB executor fixes matter |
| `docs/LANGUAGE_SPEC.md` §§5.2, 5.3, 5.7 | lexical binding, ordinary return, await expression completion, detached spawn where supported |
| `docs/ARCHITECTURE.md` §6 | stale blanket generator rejection conflicts with current tests/code |
| `docs/V1_SCOPE.md` deferred runtime section | historical deferrals: upvalues (now completed as VM snapshots), generator restoration, SpawnThread, spawn_task body |
| `docs/VM_INTERPRETER_PARITY_MATRIX.md` | narrow generator evidence; overly broad spawn status; callback bridge and 32-entry bound |
| `docs/VM_INTERPRETER_MIGRATION_PLAYBOOK.md` | preserve strict VM and dual gates; no blanket interpreter migration |
| `docs/CONCURRENCY.md` | explicitly legacy v0.9; stale-section map below |
| `docs/OPTIONAL_TYPING_DESIGN.md` | promise/await inference conservative; no typing expansion in Phase B |
| `docs/RELEASE_PROCESS.md` | compatibility-impacting corrections need notes and contract updates |
| `docs/NATIVE_API_SECURITY_POSTURE.md` | explicit capabilities; scheduler deadline is not task cancellation or a sandbox |
| `src/ast.rs`, `src/parser.rs` | FuncDef/Function flags, Yield/Await expressions, Spawn statement; no syntax needed |
| `src/compiler.rs`, `src/bytecode.rs`, `src/vm.rs` | lowering and two distinct suspension implementations |
| `src/interpreter/{mod,value,environment,async_runtime}.rs` | copied scopes, shared value components, AST execution and executor lifecycle |
| `src/interpreter/native_functions/async_ops.rs` | caches, tasks, timeout/cancel, native I/O, mapper execution |

Test inventory: search all `tests` and runtime unit tests for generator, yield,
resume, async, await, promise, spawn, channel, closure, scope, and parity. Relevant
behavior lives in `tests/interpreter_tests.rs` (generator, iterator, promise,
spawn snapshots, parallel-map groups), `tests/vm_interpreter_parity_surfaces.rs`
(arity, straight-line generators, async captures/imports, spawn),
`tests/imported_vm_callback.rs` (results, capabilities, recursion, async bridge),
`tests/native_api_security_boundaries.rs` (spawn policy inheritance),
`tests/database_lifecycle.rs`, and `tests/http_route_callback_closure.rs`.
Runtime unit groups in `src/vm.rs` test async return, reawait, nested calls,
nonpromise await, scheduler contexts, and pending promises. Unit groups in
`async_runtime.rs` test nested-runtime-safe waits; `async_ops.rs` tests native
file promises, errors, caching, limits, and mapper ordering. Compiler/parser
unit tests cover flags and scope. Documentation/fixture contracts are useful
release gates, not proofs of lifecycle completeness. `tests/generators_test.kujo`
explicitly records the loop limitation and contains historical immutable-local
mutation examples; `tests/test_generators.kujo` and iterator fixtures must not
be assumed green based on their existence. Validation results are recorded below.

## Current implementation: generators

### Pipeline and state

`func*` → `Stmt::FuncDef { is_generator: true, ... }` (or
`Expr::Function`) → `BytecodeChunk.is_generator` → `Constant::Function` /
`MakeClosure` → call dispatch builds `Value::BytecodeGenerator` rather than
running the body. Normal function declaration lowering does **not** emit
`MakeGenerator`; that opcode exists as a separate low-level conversion path
and initializes no call frames. `yield expr` → `Expr::Yield` → expression
bytecode + `Yield`; compiler also marks the chunk generator. A following
expression-statement `Pop` requires the yield operand to remain on the stack.

Anchors: parser `parse_func_with_async`/`parse_func_expr_with_async` (917/1004),
compiler FuncDef (655), Yield (1561); bytecode generator opcodes (310);
VM call dispatch (6034), MakeGenerator (5176), `generator_next` (8640).
`ResumeGenerator` delegates to `generator_next`; ordinary `Yield` execution
outside that helper errors. Struct generator methods are explicitly rejected
by both runtimes and stay unsupported.

`GeneratorState` (VM 349) contains IP, operand stack, serialized frames, chunk,
locals, captured cells and binding kinds, exhausted flag. Each `CallFrameData`
holds return IP, stack offset, named locals, local slots, initialization and
mutability vectors, and captured cells/kinds. Captures are
`Arc<Mutex<Value>>`; cloning a generator value shares its state Arc. The Rust
state struct is itself Clone, so implementation changes must not accidentally
clone that state instead of its identity.

Resume swaps in cloned generator IP/chunk/stack/frames, dropping the state lock
before execution. Frames are restored with `prev_chunk = None`, `is_async =
false`. Caller IP/chunk/stack/frames are restored only in explicit yield/return
branches. Error returns (`?` and unsupported opcode) and instruction-end fallthrough
do not follow that restoration path. No Failed/Running state exists; after error
the stored continuation is not reliably advanced or terminal. This is a source-
confirmed recovery hazard, not proof of an exploitable invalid-memory access.

The partial dispatch supports loads/stores, basic arithmetic/comparison,
stack operations and jumps. It does not implement ordinary Call, closure
creation, handler operations, or the complete VM instruction set. Consequently
nested function/generator calls and deep generator call stacks cannot be inferred
from straight-line tests. It saves local slots and named locals but not the VM's
legacy VM-wide upvalue vector, exception-handler stack, function-call bookkeeping, recursion
accounting, global scope state, or scheduler contexts. No lexical block
continuation exists independently of bytecode IP. A loop-backedge scan anywhere
in the chunk forces exhaustion at its first yield, even before an unrelated
later loop. This is an explicit parity workaround, not target behavior.

Interpreter declarations create `GeneratorDef(params, body)` without a captured
environment. Calling clones the **caller's** environment and adds parameter
scope; generator definition capture is therefore not equivalent to lexical
closure capture. Instance `Generator { env: Arc<Mutex<Environment>>, pc,
is_exhausted, ... }` stores progress by value. `generator_next` (7293) clones
that environment, executes one top-level statement at a time, uses
`Value::Return` for both yield and return, then restores the caller. A yield
inside a compound statement advances past the entire statement: remaining
branch/loop statements are lost. Non-Return errors terminate and are overwritten
when caller return state is restored; yielding an error value is a different
path and can propagate through consuming expressions.

### Identity, iteration, completion

- VM aliases consume one shared progression. No Running guard prevents two VMs
  from loading the same snapshot after the mutex is released. Language spawn
  does not currently transfer generators; concurrent resume through host/shared
  APIs is a design risk, not a tested ordinary-language race.
- Interpreter clones copy PC/exhaustion while sharing the environment. `for`
  consumes a local copy, so iterating an original binding again can restart its
  control position with already-mutated locals. Neither model is a sound
  documented single-consumer contract across both runtimes.
- VM `__vm_for_iterable` (6714) exhausts a BytecodeGenerator into a Vec before the
  consumer loop starts. It is eager, so an early consumer `break` does not bound
  producer work. Removing the loop-exhaustion workaround without fixing this
  path would turn formerly short programs into unbounded allocation/execution.
- Interpreter `for` calls generator_next incrementally on its local generator
  copy. Exhaustion is `Option::None`; a yielded null is `Option::Some(null)`.
- VM Return/ReturnNone terminate and discard the generator return payload;
  interpreter Return is interpreted as an additional yield. Subsequent resumes
  of the same exhausted VM instance return None. Interpreter behavior depends
  on which progress copy is resumed.
- Instance state is owned by its value, not the creating VM frame. Captured cell
  lifetime is now covered by Agent 1's owned VM snapshots in the tested yield
  paths; complete continuation remains Phase B. Neither implementation
  clears all retained data at exhaustion; environment/stack references can keep
  secrets and host handles live until last owner drops. Arc cycles are possible;
  no leak was measured here. Despite its name, `LeakyFunctionBody` uses a
  reference-counted store: Clone increments the stored count, Drop removes its
  entry on the last owner, and get returns an Arc-backed body reference
  (`value.rs:213–258`). Its name alone is not evidence of an AST leak.

### Generator semantics matrix

All intended entries are Phase-B proposals, not shipped claims. H/S/I/U retain
the original audit's hard/soft/independent/unknown dependency labels; the current
wave-1 classifications below supersede them.

| Scenario | Interpreter now | VM now | Parity/evidence | Intended | Dependency |
| --- | --- | --- | --- | --- | --- |
| one yield | Some then None | Some then None | E | same | I |
| multiple yields | sequential | sequential | E/T | same | I |
| local survives yield | environment retained | slots retained | T | preserve binding | S |
| mutation survives yield | retained between statements | retained slots | T | preserve mutation | S |
| nested scope survives yield | skips compound continuation | partial dispatch, no full scope snapshot | C, no | resume exact scope | H |
| yield inside loop | first yield, skips rest of loop | first yield exhausts chunk | T, limited parity | all iterations, lazy | S |
| yield inside if | skips remaining branch (1,3) | tested branch resumes (1,2,3) | T, no | resume branch remainder | S |
| yield inside try | no handler continuation | handler opcodes unsupported | C, no full support | preserve handler | H |
| error after yield | standalone error can disappear | error returns without caller restoration | T/C, no | terminal failure, restore caller | I/H |
| completion | local progress exhausted | shared state exhausted | E/C | terminal Done | I |
| resume after completion | None for same progress; copies restart | None | T/C, no alias parity | always None | I |
| captured variable across yield | caller environment snapshot | capture cells if initialized correctly | C/U | lexical identity | H |
| nested closure in generator | AST supports creation, continuation limited | MakeClosure unsupported on resume | C, no | closure outlives suspension | H |
| generator returned from function | instance retains caller snapshot | instance owns saved state | C, identity differs | preserve lexical state | H |
| generator passed as value | copied progress | shared progression | C/T | one identity | H |
| generator iteration twice | control restarts on a copy | second iteration empty | T, no | second empty | I |
| deep generator stack | ordinary AST calls, no nested continuation | Call unsupported | C | bounded normal call limits | H |
| explicit return value | appears as yield | discarded on completion | T, no | completion, no yielded return | I |
| generator method | explicitly rejected | explicitly rejected | E | retain rejection | I |
| concurrent/reentrant resume | no atomic Running state | snapshot lock released before run | C/U | deterministic busy error | H |
| independent instances | own progress/env snapshots | distinct state Arcs | C | independent locals | H |
| early consumer break | lazy producer, isolated global snapshot | producer already drained; test observes both increments | T/C, no | stop at consumer demand | I |

## Current implementation: async/await

`async func` → AST is_async flag → chunk is_async → ordinary VM closure/call.
VM CallFrame.is_async controls Return/ReturnNone wrapping: the body runs on the
calling VM before an already-resolved oneshot-backed promise is pushed. An await
inside that body can suspend the **calling execution context**. The compiler
also marks chunks async on encountering Await. This is not independently
scheduled language-function execution. Anonymous async functions and named
async captures use the same flag mechanism; capture correctness remains tied
to the closure implementation.

Interpreter `AsyncFunction(params, body, captured_env)` call paths
(`call_user_function`, 2132; expression Call, 5830) clone capture/caller
Environment and arguments, clone capability policy and optional vm_globals,
then immediately submit an AST-evaluating Tokio task. Body start is eager
submission with scheduler-dependent start time; no await is required to start
it. Captured environment is copied back on completion, so overlapping invocations
can overwrite one another's mutations. Sequential increment tests do not prove
concurrent shared-cell semantics. New interpreter contexts do not inherit every
field: output sink, source metadata, module-loader cache and stack counters are
not a complete parent snapshot.

Promise (`value.rs:1190`) is an Arc-shared oneshot receiver plus separate mutexes
for is_polled, cached Result<Value,String>, and optional join handle. Producer
is one-shot; sequential consumers re-use cached success or failure. Dropped
producer becomes a cached channel-closed failure. Interpreter language-body
errors are commonly sent as `Ok(Value::Error/ErrorObject)`, unlike native
rejection `Err(String)`. VM language-body errors can occur during call itself,
before a promise exists. This must be aligned without changing stable CLI error
classes casually. Separate cache/receiver locks leave an interval in blocking
await where a second concurrent consumer can take the dummy closed receiver;
source-confirmed race candidate, not stress-validated in Phase A.

`AsyncRuntime` is one lazy static Tokio multithread runtime with one worker and
an 8 MiB worker stack. It is reused by language interpreter tasks and native
async operations. `block_on` uses block_in_place inside a multithread runtime,
a scoped OS thread for a current-thread runtime, or the shared runtime directly
outside Tokio. Do not propose replacing a nonexistent runtime-per-await design.
VM Await (5212) uses try_recv and saves `VmExecutionSnapshot` on pending promises in
cooperative mode, rewinding IP; scheduler resumes it. That snapshot includes
handlers, legacy VM-wide upvalues and bookkeeping missing from GeneratorState.
The legacy vector is distinct from the integrated owned closure snapshots. Blocking VM
await and interpreter await move the receiver out. Interpreter uses
AsyncRuntime::block_on; VM uses its stored Tokio Handle::block_on directly
(5327), without the interpreter's nested-runtime guard. VM construction takes
the ambient handle when available, otherwise the shared executor handle.
The blocking branch therefore needs an embedding/nested-runtime regression
before claiming panic safety. Normal cooperative VM entry uses try_recv and
does not take this branch.

Native HTTP/file/sleep operations submit real async work. AsyncRuntime-safe
blocking adapters around synchronous PostgreSQL calls avoid known nested
runtime panics; current MySQL operations reuse the persistent executor (see
CHANGELOG and database.rs). Synchronous process/DB/native calls do not become
nonblocking merely because the caller is async. CPU-heavy AST evaluation can
occupy the one worker; native parallel_map uses separate bounded worker paths
and must not be used as evidence for general language async scheduling.

Imported AST async functions execute through interpreter dispatch even when
called from a VM. VM callbacks are bridged with shared globals/cells and caller
capability policy; `tests/imported_vm_callback.rs` covers this. The synchronous
32-entry thread-local bridge guard is not a global async task recursion budget.
Nested async calls work on covered cases, but spawning fresh interpreter
contexts resets per-interpreter counters: arbitrarily deep async recursion needs
a bounded task policy and subprocess tests.

No uniform structured task shutdown exists for ordinary promises: handles are
often discarded/None and dropping a promise does not abort its producer. Static
runtime/process exit is not an await-all guarantee. VM scheduler deadline bounds
the caller wait, not native producer cancellation. `cancel_task` aborts an
available TaskHandle; consuming it through await_task removes that cancellation
handle. `async_timeout` is a native promise operation, not new language syntax
or a guarantee that underlying side effects stop. No general cancel keyword or
uniform ordinary-promise cancellation contract exists.

| Scenario | Interpreter now | VM now | Parity/evidence | Intended |
| --- | --- | --- | --- | --- |
| async return | scheduled AST producer | run body then wrap | E result, C start differs | same value, explicit start policy |
| await value | identity | identity | T/E | retain |
| await promise | blocking wait | cooperative or blocking | E | expression waits; scheduler can progress |
| await same promise twice | cached clone | cached clone | T/E | retain, extend to concurrent consumers |
| async error | often Ok(Error value); call continues | fails before promise exists on tested undefined name | T/C | rejected terminal promise; catch at await |
| nested async call | scheduled child plus blocking wait | same context, wrapping | E | bounded, correct result/error |
| multiple pending async calls | independent submitted AST tasks | native pending calls; language calls not independent | C/E native | independent contexts under explicit limits |
| lexical capture | snapshot plus writeback | captured cells | E sequential, U overlap | Agent-1 lexical contract |
| mutation across boundary | overlapping writebacks may lose updates | caller/capture mutation before return | C | explicit ownership, no lost capture updates |
| imported callback | AST/VM bridge | bridge to AST async producer | E | preserve policy/globals/result shape |
| database operation | blocking work/runtime-safe wrappers | shared natives from caller | E lifecycle/C scheduling | preserve handle lifecycle and policy |
| HTTP operation | native async producer | same producer, cooperative wait | E/C | retain limits and destination policy |
| shutdown pending task | not joined universally | scheduler wait != task draining | C | documented detach/cancel policy |
| concurrent reawait | dummy-receiver race candidate | cooperative cache path differs | C/U | one terminal result for all waiters |
| async recursion | fresh contexts can reset depth | same VM depth until bridge | C/U | bounded contexts, no panic |

## Current implementation: spawn

`spawn { body }` → `Stmt::Spawn` → compiler (976) compiles body in a new compiler,
adds function constant, emits `MakeClosure`, **then Pop**, without Call or
SpawnThread. No dedicated spawn opcode exists. Comments suggesting runtime
thread spawning are not implemented behavior. VM body effects and errors do not
run, including spawn inside functions, closures, and async bodies.
The spawn compiler initializes used_locals but does not run the ordinary function
free-variable/upvalue analysis. Adding a Call or opcode alone cannot establish
correct lexical capture for this path.

Interpreter Spawn (4924) calls `capture_spawn_bindings` (497), traversing all
scope values and converting supported ones through `SpawnCapturedValue` (134).
It starts `std::thread::spawn` with cloned body and capability policy, creates
a new interpreter, registers builtins, defines transferred bindings, evaluates
the body, and discards both the result and JoinHandle. There is no joinable
value returned by this statement, error channel, or guaranteed shutdown wait.
User errors disappear; a Rust panic may reach the host thread panic hook, which
is not a language error-reporting contract. There is no local task/thread limit.

Transferred values are copied scalars, strings/bytes, recursively convertible
containers/tagged/Result/Option/struct values, Secret strings rewrapped as Secret,
and native function names. Ordinary functions/closures, promises, generators,
channels, databases, file/socket/HTTP stream handles and other unsupported
values are omitted, including whole containers with a nontransferable member.
Therefore parent-channel examples do not describe this spawn path. Native
`shared_*` state is process-global and explicitly shared; globals/lexical locals
are otherwise snapshot-isolated. Binding kinds are not carried: child define
makes them mutable. Flattening supported bindings can also leave an outer value
when an unsupported inner binding shadows it. These are correctness concerns;
no broader host capability grant follows just from losing binding mutability.

The transfer allowlist avoids moving arbitrary host handles across OS-thread
spawn, but AST async paths clone ordinary Value directly and require their own
handle audit. Arc/Mutex guarantees do not establish application-level ownership
of DB sessions or streams. Retained aliases obey the separate close/lease
contracts. Secrets remain redacted Value::Secret but are copied into new backing
strings; no secure-erasure guarantee is present.

Native `spawn_task` (async_ops 1905) accepts Function/AsyncFunction, ignores
params/body/env, checks cancellation, sleeps 1 ms and returns Null. VM-defined
BytecodeFunction is not accepted. `await_task` consumes one JoinHandle into a
promise; a second await_task call fails as already consumed, although reawaiting
the first promise can work. Full evaluation requires callable dispatch for AST
and bytecode, captured binding ownership, args/arity contract, globals/imports,
capability policy, output/error provenance, bounded scheduling and completion
ownership. Reusing trusted Interpreter::new would be wrong. It must not be
implemented by copying the current placeholder's acceptance path blindly.

Channels are Arc<Mutex<(SyncSender<Value>, Receiver<Value>)>>. Both VM send
(6751) and interpreter channel_send (6794) hold the shared channel mutex across
blocking sender.send. At full capacity a receiver needing that same mutex cannot
drain the queue: this is a source-confirmed deadlock mechanism requiring a bounded
subprocess regression, not a Phase-A hang test. Interpreter receive polls with
1 ms sleeps until data; VM receive (6766) uses try_recv once and returns null on
empty. Thus channel receive behavior also diverges independently of spawn's
transfer omission. Separate sender/receiver ownership and specify empty receive
semantics before enabling channel transfer. No broad channel rewrite is included.
Spawn is intended to allow detached parallel work on interpreter OS threads;
VM spawn currently gives neither CPU parallelism nor async concurrency.

| Scenario | Interpreter now | VM now | Proposed intended | Risk/evidence |
| --- | --- | --- | --- | --- |
| print/work | detached OS thread runs body | body discarded | schedule bounded detached work | C/T |
| reads outer binding | transferable snapshot | no body | same snapshot | E, closure dependencies |
| mutates outer binding | local change, no writeback | no body | retain isolation | E; shared_* explicit |
| uses parent channel | omitted from snapshot | no body | explicit supported transfer or rejection | C, legacy docs wrong |
| errors | result ignored | body not evaluated | bounded observable error sink | C/T |
| host effect | cloned policy gates natives | no body | identical or narrower authority | E capability test |
| multiple blocks | unbounded detached threads | multiple discarded closures | enforce task limit | C |
| inside function | snapshots visible transferable values | no body | same capture rules | C |
| inside closure | function values omitted, data copied | no body | transfer audited closures only | C; transfer policy remains open |
| inside async | new OS thread with async context snapshot | no body | common policy and limits | C |
| async inside spawn | locally declared async can run; parent async function omitted | no body | explicit callable transfer, same policy | C, targeted test pending |
| exits before child finishes | no join guarantee | nothing pending from spawn | explicit detached shutdown | C |
| spawn_task | placeholder Null | bytecode callable rejected | real callable completion | T |

## Confirmed gaps and workarounds not to preserve

G1: partial generator dispatch and missing caller restoration on errors/end.
G2: loop-wide first-yield exhaustion, statement-level continuation, eager VM for.
G3: inconsistent identity, error delivery and explicit-return semantics.
A1: VM/interpreter language async execution starts at different points.
A2: snapshot/writeback captures and non-atomic multi-consumer promise polling.
A3: per-context recursion/task limits and shutdown are not a common task model.
S1: discarded VM spawn, placeholder native spawn_task, filtered thread captures.
S2: dropped spawn errors, unbounded detached threads, lost binding metadata.

These are bounded source/test findings. They do not mean every async operation
is blocking, every generator fails, or current capability policy is bypassed.
Whole-environment clones, retained scopes, copied progress, discarded tasks and
blocking that masks missing scheduling must not become the new design merely
because a happy-path test passes.

## Closure/upvalue dependencies after wave 1

| Surface | Classification | Current requirement |
|---|---|---|
| Generator locals across yield | UNCHANGED | Preserve slots/kinds/initialization and complete scope/handler continuation. |
| Generator closure capture | SIMPLIFIED BY AGENT 1 | Reuse explicit descriptors and owned snapshot cells; add complete resume dispatch. |
| Generator frame restoration | CHANGED BY AGENT 1 | Retain capture Arcs and lexical self; lazily rebuild captured_slots. |
| Grandparent capture | SIMPLIFIED BY AGENT 1 | Compiler forwards descriptors through intermediate functions; each new closure snapshots. |
| Async closure capture | SIMPLIFIED BY AGENT 1 | VM ownership is explicit; interpreter environments and overlapping invocation writeback still need work. |
| Async mutable capture | CHANGED BY AGENT 1 | Aliases of the same closure share state; siblings remain independent. Specify concurrent mutation without inventing transaction atomicity. |
| Spawn lexical capture | NEW DEPENDENCY DISCOVERED | Separate spawn compiler still bypasses ordinary capture setup; reuse it only with an approved transfer contract, including import behavior. |
| Spawn shared state | UNCHANGED | Data snapshots remain isolated; shared_* and any admitted handles require explicit transfer semantics. |
| Frame lifetime assumptions | CHANGED BY AGENT 1 | Captures already own heap cells. Moving/restoring frames must preserve owners, not close stack pointers. |
| Scope teardown | SIMPLIFIED BY AGENT 1 | Escaped snapshot owners survive; continuation still must restore lexical scopes correctly. |
| Cross-runtime callbacks | UNCHANGED | Retain shared callback cells, globals, policy, arity, errors and the synchronous bridge guard. |
| Open-upvalue promotion/closing | NO LONGER NEEDED | Legacy VM-wide Upvalue opcodes are not compiler-produced closures. |
| Promise terminal cache | UNCHANGED | Atomic completion/waiter registration is independent of lexical snapshots. |
| Generator error cleanup | UNCHANGED | Unconditional caller restoration is still required. |
| Capture thread portability | NEW DEPENDENCY DISCOVERED | Arc/Mutex storage does not authorize transfer of embedded host handles or imply atomic read-modify-write. |
| Async start/error/shutdown contract | BLOCKED | Resolve D1–D6 before the dependent implementation slices; no scheduler work was authorized here. |

## Target semantics and decisions

The following is a proposed minimal completion contract. Existing documented
syntax, mutability, native limits and capability restrictions are constraints.
Changes to observed broken behavior need explicit release notes and regression
expectations; characterization tests are not perpetual compatibility promises.

| Decision | Recommendation | Compatibility impact / gate |
| --- | --- | --- |
| D1 generator identity | one shared instance, single active resume, Done returns None forever | removes interpreter restart-on-copy accident; release note |
| D2 generator return/error | return completes without yielding; first failure propagates and is cached; later resume repeats failure | changes swallowed errors/return-yield; approve terminal policy |
| D3 async execution start | eager submission to a bounded execution context; call returns promise before body completion | VM call/side-effect ordering changes; owner approves before scheduler work |
| D4 spawn transfer | snapshot transferable data; preserve kinds; reject referenced nontransferable captures before scheduling | replaces silent omission; channels/closures need explicit ownership decision |
| D5 detached errors/shutdown | detached statement remains no-value/no-join; route errors to documented bounded sink; no implicit join | sink/CLI behavior must be specified before implementing; no unsolicited stdout JSON |
| D6 task API | retain TaskHandle/await_task/cancel_task names; execute zero-argument callable or fail arity; expose cached completion | current no-op fixed; repeated await_task behavior requires compatibility review |

Generators: creation binds args/captures but runs no body; first resume runs until
one yield, suspends an owned continuation (operand stack, locals/kinds, scopes,
handlers, cell references and return-chunk chain). Resume continues immediately
after yield. Yielded null is a value. Caller state restores on every path.
Completion/error releases unneeded frame resources while escaping closures retain
only needed cells. An active-resume guard rejects reentry/concurrent resume rather
than deadlocking. No yield-from or async-generator semantics are added.

Async: preserve await on plain values and sequential result reuse; a promise has
Pending/Resolved/Rejected terminal state and one producer with multiple safe
waiters. Cached errors remain errors. Reuse the persistent native executor and
existing cooperative VM scheduler and preserve the integrated capture contract;
a replacement executor is outside this completion plan. Runtime mode should not change observable return/error
or lexical identity. Scheduling order between independent tasks remains unspecified;
only program-order effects before/after an awaited dependency are stable. Snapshot
capabilities at task creation; never reopen ambient authority. Document timeout,
producer drop and shutdown separately from cancellation.

Spawn: keep detached statement syntax and data snapshot isolation. Use bounded
execution resources with explicit captured-value validation; do not wrap every
Value in a new mutex. Native shared state/channels, if admitted, are deliberate
shared resources. Do not implicitly share mutable lexical siblings across OS
threads. Any admitted handle transfer needs an explicit portability contract
without changing snapshot identity. Retain policy and error/output provenance;
process exit does not silently become a global wait-for-all. `spawn_task` provides
the existing completion-oriented API, distinct from detached spawn.

Unsupported: struct generator methods remain rejected; async generators, yield
from, new join/task/capture syntax remain outside scope. Parser acceptance of
combined flags is not a commitment to async generators. Generator-inside-async
and async-called-from-generator tests mean ordinary synchronous generators and
ordinary promises only; do not introduce async iteration.

## Security and resource lifetime

This is a scoped lifecycle review, not a repository security scan or a sandbox
claim. No capability escalation was reproduced. Interpreter async and spawn
explicitly clone RuntimeCapabilityPolicy; callback bridging passes policy.
Generator execution uses the resuming VM/interpreter policy rather than storing
a dedicated creation policy in GeneratorState: cross-policy host embedding must
be tested before permitting unrestricted migration of generator values.

| Feature | Creation/live owner | Completion/error owner | Drop and shutdown |
| --- | --- | --- | --- |
| VM generator | returned Value owns state Arc; saved frames own locals/cells | resume sets exhausted on return, errors lack unified cleanup | last Arc drops fields; completion alone retains stack/captures; cycles need audit |
| AST generator | value owns copied PC plus shared env/body | local iteration copy advances/exhausts; caller restored | other copies can retain env; body allocation policy separate |
| ordinary language promise | caller owns receiver/cache; executor owns producer | oneshot send, waiter caches; AST errors may be values | dropping receiver does not cancel detached producer; process exit may cut work short |
| native promise | executor/native resource owner plus receiver | native Result sent and cached | timeout is not universal abort; retained handle/resource follows native Drop |
| detached spawn | OS thread owns body/snapshot/policy | result discarded, no completion observer | detached handle dropped immediately; thread outlives lexical parent; process exit terminates remaining work |
| TaskHandle | Arc<Option<JoinHandle>>, cancellation flag | await_task takes handle; cancel_task aborts if still held | consumption changes available cancellation; no common cached task completion |
| cooperative VM context | scheduler owns saved state and globals | scheduler completion/error removes/resumes context | timeout cleanup and underlying native task lifetimes require distinct tests |

Security acceptance cases:

1. Authority must never increase across async/spawn/imported callback boundaries.
   Test denied FS, process, shell, network, database, env and secret-related output
   in each executable path. Clone/intersect policy explicitly; do not use trusted
   constructors at executor boundaries. Existing spawned-interpreter denial test
   is evidence only for that path, not a future VM spawn implementation.
2. Tasks can outlive lexical parents and keep their **owned policy snapshot**;
   this is not automatically an escalation. Parent timeout/drop does not prove
   side effects have stopped. Document it and test shutdown with a subprocess.
3. Suspended frames, caches and environment cycles can retain secrets/handles as
   long as reachable. Secret wrappers remain redacted; do not promise zeroization
   or absence of cycles without allocation/drop instrumentation.
4. OS spawn excludes nontransferable handles. AST async does not apply that
   allowlist. Verify DB close/lease aliases, file/network stream ownership and
   closure portability before broadening transfer. Unsupported transfer must
   report a value/path diagnostic without dumping secrets.
5. VM generator unsupported-op/error paths leave execution fields displaced;
   poisoned Mutex unwraps can panic. No user-triggered memory-safety exploit is
   established. Use malformed-state unit tests and catch/recover program tests,
   not a broad unsafe claim.
6. Limits are needed for task count, detached threads, suspension state size,
   async recursion and queue pressure. Existing parallel_map concurrency limits
   do not cap all language async calls or spawn blocks.

## Performance analysis and benchmark plan

No timings below are claimed for production; this audit identifies operations
and a reproducible Phase-B measurement design.

| Hot path | Current cost / risk | Measure after semantic correctness |
| --- | --- | --- |
| generator creation | clone chunk/locals/slots/capture maps; interpreter clones all scopes | allocations and retained bytes with 0/10/100/1000 live locals |
| VM resume/yield | clone caller stack/frames/chunk, clone generator stack/frames, repeat at yield; scan entire chunk for JumpBack per yield | ns/yield, allocations/yield, bytes copied vs caller depth and chunk size |
| generator for | collect entire sequence to Vec before body | peak RSS for 1/1000/100000 yields; early break must execute one producer step |
| AST resume | body.get and whole environment clone/save | ns/yield vs unrelated globals and nested scopes; inspect body allocation behavior |
| async call | interpreter env snapshot, task/oneshot and Arc/Mutex allocations; VM inline body then promise | empty/immediate result, native pending, nested call, capture sizes |
| await | receiver/cache locks, blocking lanes or cooperative snapshot/poll | reawait cached vs pending; 1/10/100/1000 waiters; scheduler fairness |
| executor | reused one-worker Tokio; blocking code can occupy worker; scoped threads for runtime-safe blocking | task queue latency, worker/thread peak, DB blocking vs native async file I/O |
| spawn | OS thread per block, recursively copied snapshot; no global limit | spawn/join-through-native completion cost and resource ceiling |
| channels/shared state | Arc/Mutex and blocking channel calls | contention, throughput, deadlock watchdog at bounded producer/consumer counts |

Use release builds, fixed local fixtures, deterministic inputs, warmup and at
least 20 samples; report median/p95 and peak memory/thread counts, hardware,
commit and build flags. Separate cold executor initialization from warm paths.
No public HTTP/DB service dependency; use temp files and optional locally
provisioned database test lanes. Correctness gates must not assert wall-clock
speedup; benchmarks record regressions against the rebased baseline. Avoid
one-mutex-per-value redesign and whole-global snapshot growth in the common path.

## Phase-B exact test plan

New characterization tests deliberately assert current deficiencies. Replace
those assertions with approved intended outcomes in the same fixing slice;
never delete an observation merely to make the gate green. Add no permanently
failing or silently ignored target tests.

### Generator cases

| ID | Program/fixture | Target oracle |
| --- | --- | --- |
| G01 | `func* g(){yield 7}`; resume 3 times | Some(7), None, None |
| G02 | three straight yields incl null | Some(1), Some(null), Some(3), None |
| G03 | `mut n:=2; yield n; n+=3; yield n` | 2,5; original binding kinds preserved |
| G04 | nested if with local shadow and two yields | both yields in order; outer binding unchanged |
| G05 | while/for/loop with yield before increment | full bounded sequence; no skipped remainder |
| G06 | yield before a later loop in same chunk | first yield must not exhaust unrelated loop |
| G07 | try { yield 1; throw(...) } except { yield 2 } | 1,2; handler and caller both restored |
| G08 | yield then uncaught error; caller catches and calls normal function | one failure; normal call returns 42; repeat follows D2 |
| G09 | return 9 after yield 1 | only 1 yielded; completion payload not yielded |
| G10 | two instances interleaved, plus alias | instances independent; alias shares progression |
| G11 | returned generator captures parent and grandparent mutables | preserve that generator's owned snapshot cells across resume/creator return; siblings remain independent |
| G12 | closure created inside generator escapes, called across yields and completion | new closure snapshots current values; its aliases retain mutation without stale slot access |
| G13 | generator passed to a function; iterate twice | second consume empty; no progress copy |
| G14 | ordinary nested helper calls plus nested synchronous generator iteration | full dispatcher behavior; bounded call depth |
| G15 | first consumer break on infinite producer | exactly one requested yield, bounded memory |
| G16 | reentrant/concurrent host resume using barriers | one active resume, other deterministic busy error |
| G17 | invalid slot/state unit fixture, error in resume | no panic, caller restored, no out-of-bounds access |
| G18 | scope teardown and dropped generator with captured handle sentinel | prompt release unless escaping closure retains cell |
| G19 | struct generator method | deterministic existing unsupported error |

### Async cases

| ID | Fixture | Target oracle |
| --- | --- | --- |
| A01 | return 42, implicit null, await 7 | 42, null, 7 |
| A02 | same promise awaited sequentially and concurrently through barriers | one producer, same result/error for all waiters |
| A03 | body throws before first await / after native await | call gives promise; catch at await under D3 |
| A04 | inner returns 21, outer awaits and doubles | 42, nested contexts cleaned |
| A05 | two pending calls, controlled completion order | independent progress; no timing-based assertions |
| A06 | captured immutable binding and same-closure mutable counter, overlap gated by barriers | immutable denial; explicitly specified concurrent alias mutation; sibling snapshots remain independent |
| A07 | imported async invokes VM closure and VM invokes imported callback | return/dict normalization, globals, mutation, error, arity |
| A08 | producer drops sender and producer panics in host fixture | terminal rejection cached; no hang |
| A09 | SQLite lifecycle plus optional PostgreSQL/MySQL local async context | close/alias semantics, no nested-runtime panic |
| A10 | native HTTP using loopback fixture and private-net denial | output shape/limit preserved, policy deny enforced |
| A11 | subprocess exits/times out with pending producer | documented detach/cancel outcome; no claim parent wait aborts producer |
| A12 | restricted capability matrix across task/callback boundaries | no effect before denial; stable error status |
| A13 | nested async recursion beyond configured context limit | bounded error, no Rust stack overflow/thread explosion |
| A14 | timeout then reawait source; cancel before/after await_task takes handle | explicitly approved caching/cancellation semantics |
| A15 | native process/DB blocking work alongside pending timer | correctness and bounded resources; benchmark fairness separately |

### Spawn and cross-feature cases

| ID | Fixture | Target oracle |
| --- | --- | --- |
| S01 | spawn writes unique shared marker; completion handshake | body runs exactly once in both runtimes |
| S02 | read scalar/collection, mutate child copy | observed capture value; parent unchanged |
| S03 | nested lexical shadow incl unsupported inner value | nearest binding or explicit transfer error, never outer fallback |
| S04 | parent channel used by child | supported transfer works or deterministic preflight rejection per D4 |
| S05 | body throws, multiple blocks with one failure | error sink gets one failure, other work completes |
| S06 | restricted file/process/net/DB effects | inherited deny; no new trusted context |
| S07 | spawn inside function and inside closure | captured data valid after lexical parent returns |
| S08 | task count above limit, secrets and nontransferable handles | bounded error, no secret text, no silent omissions |
| S09 | shutdown before completion in subprocess | exact approved detached policy; bounded harness kill |
| S10 | spawn_task of AST/bytecode callable; nonzero arity | actual 42 or arity error, never placeholder null |
| S11 | await_task twice and cancel race with controlled barriers | approved consumed/cached completion, no indefinite wait |
| S12 | full channel with active consumer; empty channel receive | no mutex-held-send deadlock; approved wait/null parity; bounded subprocess |
| X01 | ordinary generator consumed inside async | ordered generator values, awaited aggregate |
| X02 | generator calls async helper and awaits ordinary promise | resume/caller state survives; no async generator syntax |
| X03 | closure inside generator; generator returned by closure | each created closure snapshots once; its retained cells survive yields and creator return |
| X04 | spawn uses closure | D4 transfer policy, never silent no-op |
| X05 | spawn declares/invokes async; async spawns work | capability and lifecycle policy unchanged across both boundaries |
| X06 | async imported callback mutates captured state | Agent-1 cell identity retained, errors catchable |

Cross-feature tests are conditional on the approved transfer/suspension contract;
unsupported combinations must reject explicitly. No test should accidentally
commit Kujo to async generators or struct generator methods.

### Differential harness

Use a Rust integration harness invoking the CLI in temp directories for both
`run` and `run --interpreter`, plus low-level VM/interpreter tests for state
identity and save/restore internals. Compare normalized stdout, explicit
serialized return/result marker, exit status/error class, observable mutations,
completion count and repeat-resume behavior. Do not compare unstable human error
wording. For error cases stdout must include a post-catch sentinel proving caller
recovery. Use local promises, temp files, replay fixtures and deterministic
producer barriers; no public network, random sleeps or global reused shared keys.

Bound child processes to 5–10 seconds, kill/reap on timeout, limit captured output,
and clean test resources on every path. An optional enumerator can generate all
combinations of 0–2 yields, branch true/false, loop count 0–3, one scope shadow,
and failure before/after yield from fixed seeds recorded in the test. Keep AST
size/depth and statement budgets fixed. Report the source and runtime on failure;
this is deterministic differential coverage, not flaky random CI. In-process
helper tests are reserved for nonblocking bounded programs.

## Ordered implementation slices

| Slice | Work, entry dependency | Exit gate / rollback boundary |
| --- | --- | --- |
| 0 | branch from the verified integration base; resolve D1–D6; inspect current ownership | baseline and closure/callback parity; update this plan before source edits |
| 1 | generator owned continuation + unified dispatch + unconditional caller restore | G01–G08/G14/G17; no async/spawn redesign in this commit |
| 2 | generator identity/lifecycle, handlers, cell retention, lazy for | G09–G19 and cross-closure tests; remove loop workaround only with lazy iteration |
| 3 | atomic promise terminal state and waiter registration | A01/A02/A08/A14; preserve native API errors, independent of scheduling change |
| 4 | language async scheduling/context parity, captured cells/policy | A03–A07/A09–A13/A15; preserve persistent executor and callback limits |
| 5 | transfer contract and real native spawn_task callable execution | S02–S04/S06–S08/S10/S11; bounded resources, no trusted-context fallback |
| 6 | VM spawn lowering/execution plus detached errors/shutdown | S01/S05/S09, interpreter parity; choose opcode only after shared context design |
| 7 | supported cross-feature parity, performance and docs/release alignment | X01–X06, all gates, fresh evidence; no unsupported syntax promotion |

Each slice must include source tests and contract/docs changes, be independently
reviewable, and stop if it would recreate Agent-1 capture infrastructure. Do not
use a new SpawnThread opcode as a substitute for specifying task ownership.

## File ownership and conflict map

| File | Expected Phase-B work | Conflict risk |
| --- | --- | --- |
| `src/compiler.rs` | generator iteration, spawn lowering, capture metadata | HIGH; preserve integrated captures |
| `src/vm.rs` | continuations, frame teardown, dispatch, await, spawn | HIGH; preserve integrated captures |
| `src/interpreter/value.rs` | generator identity, promise/task state, capture representation | HIGH |
| `src/interpreter/environment.rs` | retain v1 snapshots while addressing invocation ownership | HIGH; Agent 1 did not redesign interpreter environments |
| `src/interpreter/mod.rs` | AST continuation, async context, transfer and errors | HIGH |
| `src/bytecode.rs` | minimal spawn/iterator opcode changes if needed | MEDIUM/HIGH metadata conflict |
| `src/ast.rs`, `src/parser.rs` | no syntax work expected; unsupported combination diagnostics only if required | LOW; avoid incidental edits |
| `src/interpreter/async_runtime.rs` | bounded task/context integration, blocking boundaries | MEDIUM |
| `src/interpreter/native_functions/async_ops.rs` | promise cache, real tasks, cancellation/timeout lifecycle | MEDIUM |
| `src/interpreter/native_functions/database.rs` | verify runtime-safe ownership; avoid redesign | LOW/MEDIUM |
| `src/main.rs`, `src/cli_output.rs` | only approved shutdown/error sink integration | MEDIUM stable CLI contracts |
| new characterization/continuation test files | isolated tests; update current assertions per fixing slice | LOW |
| existing parity/security/callback suites | extend narrowly without weakening expectations | MEDIUM concurrent additions |
| docs listed in next section | semantics/release evidence | LOW; preserve historical ledgers |

Phase-A ownership is the new plan, a standalone characterization test file, and
its validation ledger. No shared implementation files are changed.

## Starting Phase B after integration

1. Fetch and create `runtime/generator-async-spawn-completion` from the exact
   [handoff base](GENERATOR_ASYNC_SPAWN_PHASE_B_HANDOFF.md), or verified main after
   integration. Preserve the audit branch as Phase-A history.
2. Read `CaptureSource`, `resolve_capture`, `finish_child`, scoped/imported binding
   handling, `lexical_self`, `capture_slot` and `requires_closure_vm` in the new tree.
   Do not copy obsolete `find_free_variables` or open-upvalue recommendations.
3. Compare `GeneratorState` / `CallFrameData` with `VmExecutionSnapshot`. Preserve
   slot/kind/initialization vectors and captured Arc identities; rebuild the derived
   cache. Decide ownership of caller chunks, handlers, globals, policy and scheduler
   IDs before allowing a continuation to move between execution contexts.
4. Review interpreter async environment writeback, host-handle ownership, callback
   policy and the 32-entry synchronous bridge guard. Sequential capture tests do
   not prove concurrent mutation or a task recursion bound.
5. Run the handoff's closure/import/callback, characterization, native-security and
   workflow schema gates. Record changed characterization outcomes with their
   intentional fixing slice. Do not weaken unrelated expectations.
6. Resolve D1–D6 and implement the ordered slices. No whole-file ours/theirs merge,
   tracing collector, shared-sibling capture or replacement executor is implied.

## Documentation drift and Phase-B documentation plan

Do not promote legacy `docs/CONCURRENCY.md` to authoritative status by updating
only its introduction. These exact sections need replacement or archival links:

| Legacy section | Stale detail / current source |
| --- | --- |
| Threading Model / Thread Safety (53–85) | general thread-safe value claims omit transfer allowlist and ownership rules |
| Async/Await Architecture (86–189) | synchronous v0.9 narrative does not describe scheduled AST tasks or cooperative VM await |
| Promises / Structure / State Machine (190–265) | old representation omits current oneshot, cache and task-handle ownership |
| Channels and Spawn with Channels (266–349, 402–421) | parent channel is not transferred by current SpawnCapturedValue |
| Spawn Blocks / Implementation / Characteristics (350–401) | VM body is discarded; interpreter uses filtered snapshot and inherited policy |
| Generators / State / Execution Flow / Usage (422–547) | statement PC is not full continuation, copied identity, Option completion, no general .next guarantee |
| Fan-Out/Fan-In, Pipeline, Worker Pool (550–681) | examples rely on transfer/closure behavior not currently supported by spawn |
| Best Practices / shared mutation and promise errors (682–774) | distinguish snapshot copies, shared_* state, terminal native errors and language Error values |
| Performance / Async limitations / Generator performance (775–829) | Tokio already exists; no measured near-zero generator overhead; VM for eagerly buffers |
| Debugging / Future Improvements (831–884) | historical future Tokio/cancellation claims; native cancel_task/timeout already exist |

Wave-1 integration corrects `ARCHITECTURE.md` §6's blanket generator failure
and narrows the parity matrix's spawn row to what its test proves. Those shared
document changes occur in the integration history, preserving Phase-A ownership. Phase B should publish `docs/RUNTIME_CONCURRENCY.md` (or fully replace the
legacy article with a current guide) covering ownership, syntax, task scheduling,
await, promises, generators, spawn, channels, threading, capabilities, shutdown
and both runtimes. Link it from LANGUAGE_SPEC, ARCHITECTURE and parity matrix;
update STANDARD_LIBRARY for task API semantics, CLI contracts if error output
changes, CHANGELOG, ROADMAP and V1_SCOPE deferral status together. Preserve legacy
launch evidence and optional-typing limits. Update doc contract tests and
regenerate any impacted evidence through its generator, never by hand.

## Definition of done

Phase A: baseline recorded; all requested pipelines, matrices, dependencies,
security/lifetime/performance risks, target decisions and ordered tests captured;
characterization passes without runtime changes; final validation recorded;
small commits pushed on the isolated branch with clean working tree; durable
handoff retrievable. Failing pre-existing gates remain clearly identified.

Phase B: verified wave-1 baseline checked out; D1–D6 resolved and documented; complete owned
generator continuation, lazy iteration and identity/error semantics proven;
promises support safe repeat/concurrent await; async/task behavior and policy
align across supported runtime paths; spawn actually executes with bounded
resources and explicit transfer/error/shutdown contracts; supported cross-feature
cases pass, unsupported combinations reject; no new syntax; no capability
expansion; no unexplained performance regression; all targeted, full, VM/dual,
docs and CLI gates pass or have an explicitly accepted external limitation.

## Validation ledger

Commands ran locally on macOS with the baseline source unchanged before copying
in the new plan/tests. Logs were captured under `/tmp/kujo-phase-a`; the durable
results here do not require those temporary files to survive. No live provider
credentials or services were required by the new tests.

| Baseline command | Result | Wall seconds |
| --- | --- | --- |
| `cargo fmt --check` | PASS | 11 |
| `cargo check` | PASS | 3 |
| `cargo test` | FAIL, exit 101: docgen suite 51 pass / 4 fail; stopped subsequent integration suites | 528 |
| `cargo test --test vm_interpreter_parity_surfaces` | PASS | 7 |
| `cargo run -- test --runtime vm` | PASS, 154/154; 6 skipped of 160 discovered | 33 |
| `cargo run -- test --runtime dual` | PASS, 154/154; VM-primary 154, fallback 0; 6 skipped | 16 |

The four full-suite failures predate every branch edit:

- `docgen_external_validation_allows_cross_host_redirect_when_hosts_are_allowlisted`
- `docgen_external_validation_allows_same_host_redirect_hops`
- `docgen_external_validation_blocks_redirects_to_non_allowlisted_hosts`
- `docgen_link_validation_budget_max_external_checks_truncates_deterministically`

The first, second and fourth observed broken_link_count 1 instead of 0; the third
missed the expected blocked-redirect diagnostic. These are local HTTP docgen tests;
root cause was not established by this runtime audit. Do not call the entire
baseline green or attribute these failures to Phase A.

Dedicated fixture checks against the built baseline binary:

- `target/debug/kujo test-run tests/generators_test.kujo`: exit 3,
  KUJOPARSE001 Expected expression at 414:1 (pre-existing legacy suite failure).
- `target/debug/kujo test-run tests/test_generators.kujo`: exit 4, No tests found;
  this file is an ordinary script, not a test-framework suite. Corrected invocation
  `target/debug/kujo run [--interpreter] tests/test_generators.kujo`: both PASS.
  Its success message is not an assertion of full generator correctness.
- `target/debug/kujo test-run tests/iterators_test.kujo`: PASS, 10/10.

New tests are isolated in `tests/runtime_semantics_characterization.rs`; no old
expectation was weakened. They exercise actual state and errors rather than
repeating implementation-shaped mocks. The function names are the exact
reproduction entry points. Final validation and dedicated runtime/security suite
results follow below.

## Durable finding register

SignalBox project `kujo` admits only unresolved findings from this audit. No
completed-work summary, downstream task or handoff was stored there. Deduplication
searched generator/spawn concepts plus exact generator_next, spawn_task and
channel_send identifiers and channel deadlock; no equivalent entries were found.

| Finding | Capture ID | Human-review Signal ID |
| --- | --- | --- |
| generator continuation / identity / error semantics | `cap_2765f7c2-85a7-43eb-867d-6ecac5536b19` | `sig_ccd7855b-1c59-43b7-b782-4891c44fc40d` |
| discarded spawn / placeholder task execution | `cap_5f776ba7-853c-472f-a19f-ccf58435ed1b` | `sig_638970cf-e4f0-49cb-8050-fa9349cd760c` |
| bounded-channel locking / empty receive parity | `cap_764d9d45-8ae6-444b-bf4c-bd7903d7a25e` | `sig_668dcebc-81a5-46b2-83ae-ab5c400afcf3` |

All six exact-ID lookups succeeded. Concept queries `generator continuation`,
`placeholder`, and `channel` retrieved the corresponding Capture and Signal;
`spawn execution` additionally retrieved the spawn Signal. Duplicates skipped: 0.
Rejected from SignalBox: routine verification, completed audit summary, and
unreproduced promise-race speculation as a confirmed bug. The four docgen baseline
failures are recorded in this validation ledger without claiming a diagnosed
runtime defect. Strata receives the separate dated handoff/current-state episode
with repository, branch, commits and this document as provenance.

### Final validation (after Phase-A changes)

| Exact command | Result | Wall seconds |
| --- | --- | --- |
| `cargo fmt --check` | PASS | 46 |
| `cargo check` | PASS | 4 |
| `cargo test` | PASS, including all integration suites and doc tests; existing ignored tests remain ignored | 460 |
| `cargo test --test runtime_semantics_characterization` | PASS, 13/13 | 3 |
| `cargo test --test vm_interpreter_parity_surfaces` | PASS, 115/115 | 2 |
| `cargo run -- test --runtime vm` | PASS, 154/154; 6 skipped | 12 |
| `cargo run -- test --runtime dual` | PASS, 154/154; 6 skipped; 154 VM-primary, 0 fallback | 13 |
| `cargo test --test interpreter_tests --test imported_vm_callback --test database_lifecycle --test native_api_security_boundaries` | PASS: interpreter 263, callback 8, database 4, security 90 | 11 |
| `cargo test --test docs_examples --test readme_contracts --test architecture_docs_contract --test language_spec_contracts --test cli_contracts --test cli_json_contracts` | PASS: examples 6, README 1, architecture 1, language spec 7, CLI 30, JSON 19 | 19 |

All four baseline docgen failures passed on final rerun (55/55 in that suite),
with no implementation changes. They are intermittent observations, not confirmed
persistent defects; their cause remains undiagnosed. The separately invoked
legacy `generators_test.kujo` parse failure remains unresolved. That fixture is
not the passing Rust characterization suite. Routine generated `.out` files from
verification were removed, preserving a clean reviewable branch.

Characterization commit: `44c30b1be0e9aeb423dfd53c3d81051a2ad6a11f`.
The following documentation commit records the audit, proposed policies and final
validation. No implementation exception was needed. Phase B has not begun.
