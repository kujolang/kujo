# Experimental native owned-value lifetime mechanism

Status: **typed owned arguments connected; broader native coverage and acceptance incomplete**.
Eligible native functions now transfer named arguments into frame-local roots
after frame entry. The caller drops temporary argument vectors before invocation,
so those vectors and the callee's name map do not retain artificial COW aliases.
Immediate-only calls retain their direct payload/kind path; mixed or owned calls
use named transfer even when a cached function was originally called with scalars.
Parameter initialization errors exit before body effects and release the frame.

Named assignment uses the tagged checked helper for both immediate and owned
values. A collection constructor must not redirect a global write into the native
function's private local map or bypass the global binding's readonly check.
Regression tests warm native constructors before checking environment updates,
catchable readonly errors, effect counts and subsequent successful calls.

Nested binding writes retain immutable path shapes in the JIT compiler and pass
tagged index/value operands through explicit generated stack storage. The helper
uses the canonical checked assignment operation and a context-local depth cache;
unique rows remain reusable and retained aliases still force copy-on-write.
Named roots resolve through the checked global environment, including names used
only as mutation destinations. Locked loops use their existing environment guard.
Named writes to captured bindings, and named writes in lexical-self contexts,
remain outside loop admission because those destinations require frame cells;
they must never be redirected to a same-named global.
Temporary operands are released after successful writes or by frame cleanup on
error. Nested appends and pop/rebinding remain separate native admission work.

The canonical checked array/byte append operations also release cached weak
identities along the destination path before mutation, then remember the updated
depth bounds. This avoids relocating otherwise unique parent and array nodes
solely because the cache observes them. Real value aliases still force COW;
unrelated cached identities retain their external-mutation invalidation behavior.
Checked pop/removal follows the same rule after obtaining the conservative depth
bound. Removing weak cache observations on a fallback path changes no values;
successful shrinking operations refresh the unchanged upper bound afterward.

Regression coverage includes mixed types, four/five arguments, loops, recursion,
imported helpers, callable arguments, snapshots and failure/success recovery across
interpreter, VM and JIT. A storage test retains unique array backing storage through
128 writes after parameter transfer. The frozen candidate passes all 50 test-size
checksums in release JIT, VM and interpreter modes (150/150), with fallback
allowed. Production, platform and controlled speed gates remain open. See the
[candidate evidence](../benchmarks/langarena/owned-argument-entry-2026-10-23.json).
Nested append/pop operations and closure/exception surfaces still limit coverage.
Native entry counts are not elapsed speed evidence.

General stack index reads now use tagged operands and the VM indexing contract.
Success and error exits release frame roots; repeated owned reads have a bounded
storage regression. The [frozen stack-index candidate](../benchmarks/langarena/checksums-2026-10-23-native-stack-index-jit.json)
passes all 50 JIT test-size checksums, with fallback allowed. It predates the
separate field-error fix and does not establish production or speed acceptance.

Ordinary array literals now marshal typed elements in source order, release
consumed roots and return an independently rooted array. Construction checks the
same value-nesting limit as the VM and preserves preceding stack values on error.
The tagged stack-pop helper is registered for loop compilation as well as function
compilation. Spread-marker bytecode remains excluded from native admission.
Independent tests cover empty/mixed/nested arrays, snapshot isolation, effects,
error recovery, native loop entry and bounded repeated-construction storage.

Loop locals use tagged SSA even when initialized by collection literals. Their
normal and exceptional exits persist current values to physical VM slots; updates
do not stop at the temporary name map. Integer-map fast paths retain their separate
physical-slot access. Regressions cover arrays/dictionaries and retained snapshots.

Struct construction retains immutable name/field metadata in the compiler for
the lifetime of generated code. Typed elements become ordinary struct Values;
construction follows VM insertion order and nesting checks. Tests cover metadata
outliving its bytecode chunk, recursive builders, mixed fields, snapshots, native
loop replacement and bounded roots. Nested path writes now use the checked helper
described above; append/pop admission remains open.
See [constructor evidence](../benchmarks/langarena/native-struct-construction-2026-10-23.json).

Cold VM callbacks now use the same field resolver as ordinary VM execution,
including legacy channel/ArgParser/image/server receiver markers. Regressions
keep the caller hot and the field-using helper cold, checking markers, prior
effects, failed assignment targets and recovery. Native `FieldGet` now uses the
same resolver with compiler-retained field names and tagged result roots. Global
method lookup reads the borrowed environment without acquiring its mutex again.
Legacy receiver markers retain their physical VM stack effects. Native function
entry records the stack base; every generated function exit discards its suffix,
including unconsumed receivers, while preserving caller values. Loop regions
remain in the enclosing VM frame and retain its stack lifetime.
Locked native loops resume in the VM before legacy receiver-producing field
lookups. Resuming only at the subsequent call would discard the hidden receiver
when rebuilding the operand stack. Ordinary struct field reads stay native.

The sections below retain earlier staging evidence and the ownership contract;
statements about former entry exclusions describe those earlier stages.

Native invocation
wrappers and generated function entry/exit helpers now enter and leave lazy root
scopes. `src/native_owned.rs` is compiled with `runtime-jit`. Ordinary named loads
now use tagged roots, preserve local/global/builtin precedence and leave no hidden
values on the VM stack. Calls explicitly marshal their callable before arguments,
and checked named assignments borrow the tagged value. Specialized collection
helpers still retain legacy paths. VM-stack pops encode owned
callback results, and tagged pushes borrow roots when marshaling callback
arguments. This repairs the native `str(x)` return path from the original audit;
it does not establish full native parity/performance. Closure, generator, async
and legacy receiver-adaptation entry guards remain.

Conditions on tagged owned operands now borrow the root and use `Value::is_truthy`,
matching VM/interpreter truth rules without cloning the container. Immediate
conditions retain native evaluation. Stale roots report an error and generated
conditions exit through the existing error path. This covers callback-produced
and named empty strings and logical operations.

The named-load candidate passed the original eight diagnostic probes but failed
expanded equal-value cases: comparing distinct handle identities happens to pass
the older unequal-string probe. Native equality now branches to borrowed
`Value::equals` when either operand is owned, including mixed owned/scalar pairs.
Immediate comparisons retain their native paths. The helper validates both kinds
and identities without cloning collections or allocating new roots. This does not
close dead-operand release, legacy collection helpers or owned entry guards.

The earlier native execution census showed that owned-argument call boundaries
were heavily exercised. Legacy paths used incompatible VM-stack markers and an
append-only object vector. Converted paths now use the common lifetime mechanism;
remaining legacy paths must be converted and verified before admitting their
operations into native code.

## Storage and ownership

Guarded append lowering has a root operation for temporarily moving an
evaluated snapshot out of its handle. Existing rebinding guards can then release
that artificial alias before mutating the destination. Fallback restores the
snapshot under the same handle; successful guards consume it, and generated code
must release the consumed operand root. Both handles must belong to the current
frame and be distinct. Native guarded string append uses this operation for SSA
locals and the same checked rebinding semantics for physical locals and globals.
Its successful edge consumes both operands; its fallback edge keeps their values
and tags for ordinary addition and assignment. Guards do not invoke user code or
repeat RHS evaluation. Guarded array/byte append remains excluded from compilation.

Each native root owns an ordinary `Value`. Duplicating a root uses `Value::clone`,
so arrays, strings and other shared containers retain their existing copy-on-write
and snapshot behavior. A root is not a deep copy or an alternative heap/collector.
It must not retain unrelated globals or whole VM snapshots.

A monotonically issued 64-bit handle maps to a recyclable slot. Releasing a root
removes the mapping and frees the slot; old handles never identify replacement
values. All payload bits are significant, including the sign bit when transported
as `i64`. Payload sign or `-1` is never a type/ownership marker. The future ABI
uses owned kind `4` alongside the payload. The staged codec keeps the existing
integer/Boolean/null/float kinds `0..3` allocation-free and rejects unknown kinds.
Encoding, borrowing, taking, duplicating, promoting and releasing payloads all
use the same kind dispatch; floating-point payload bits remain unchanged.

Handles are store-local. Contexts backed by different stores must exchange
ordinary `Value`s through the codec, never copy raw owned payloads between stores.
Equal numeric handles in different stores do not identify the same root. Direct
native recursion may share the same store with separate activation frames.

Slots form intrusive per-frame lists. Insert, unlink and promotion do not scan
the frame or retained values. Frame cleanup walks only its live roots. The slot
vector and lookup capacity retain their high-water allocation for reuse and scale
with peak live roots, not cumulative loads. Frame-vector capacity follows peak
active depth. The runtime's existing recursion limits remain the caller's duty.

Handle/frame identity exhaustion returns an error before changing storage rather
than wrapping and reviving stale identities. This is a 64-bit identity-space limit,
not a slot retirement mechanism that gradually retains dead values.

| Operation | Contract |
| --- | --- |
| `enter_frame` | Returns a token for a new current frame. |
| `insert` | Moves a `Value` into a new root in the current frame. |
| `get` | Borrows a live root, including an outer frame's argument. |
| `duplicate` | Clones the value into a distinct current-frame root. |
| `take` | Removes a current-frame root and moves out its value. |
| `release` | Removes and drops a current-frame root. |
| `promote` | Transfers a current-frame return root to its immediate caller without cloning. |
| `leave_frame` | Releases remaining roots; rejects stale/out-of-order tokens without mutation. |

`NativeCallRoots` tracks logical scope depth without allocating frame storage for
immediate-only execution. The first owned value materializes the needed ancestor
frames. An immediate-only nested call cannot release its parent's existing roots.
Consuming operations check logical depth against materialized frame depth before
delegating to the store; an empty callee must not be mistaken for its caller.
Every successful generated function entry acquires a logical scope after its
shared recursion-depth guard; every generated exit releases it. Owned return roots
are promoted before that cleanup. Invocation wrappers own an outer boundary scope,
take the result into the context before releasing it, and let the VM move out the
result without an extra clone. Standard status-return and direct payload-return
calling conventions are handled separately. Reused boundaries clear old result
metadata and release any previous context-owned return.

Tests exercise both wrappers through recursive ABI probes, normal/error exits,
return transfer and 1,000 context reuse cycles. A real compiled immediate function
and a depth-rejected entry leave all root-storage capacities at zero. These probes
do not establish generated owned-operand correctness; they supply owned payloads
through the codec directly, bypassing the still-incomplete bytecode paths.

A callee may borrow an outer argument but must duplicate it before owning or
releasing a native operand. Promoting a return transfers that operand's ownership;
it does not add another reference. The outermost boundary takes the return `Value`
before leaving its frame. Rust store destruction drops any remaining values, but
generated normal/error exits still need explicit cleanup while the store is live.

## Dictionary literals

The supported `MakeDict(0)` path converts its constructed dictionary from the
helper stack into a tagged owned operand immediately. It no longer returns an
integer sentinel or leaves a hidden dictionary on the VM stack. Local stores
marshal the tagged value even when its payload is integer `-1`; slot-only local
names are included in native name tables for helper lookup. Direct-mode local
loads use the same owned codec.

Regression coverage includes returns, locals, callback arguments, truth/equality,
reassignment to `-1`, saved snapshots, recursive returns and independent subsequent
mutation. Repeated compiled literal creation/discard joins the bounded-root tests.
Nonempty dynamic-key and unique fixed-key literals now marshal tagged operands,
release consumed roots and capture the constructed owned result. Dynamic keys
follow VM string-key validation and source-order duplicate replacement. Fixed-key
vectors are retained by the compiler independently of the source chunk; returned
dictionaries acquire their own Arc reference. Both helpers retain the VM nesting
depth checks. Invalid dynamic keys use catchable VM/native errors without replay.

Repeated constant keys use the compiler's ordered-map path to preserve evaluation
order and last-value-wins semantics; string constant loading still prevents native
compilation of that particular form. Specialized indexing helper protocols and
wider owned argument entry remain separate gates. Integer literal-key coercion
still differs between interpreter and VM; this change does not resolve that policy.

## Owned operators

Native arithmetic, ordering and unary operators branch on operand tags before
entering the existing immediate paths. Owned operands call the reference VM's
operator implementations, including struct methods and canonical errors. The
helper clones ordinary Values before callbacks, holds no root-store borrow or
global lock across user code, and roots its tagged result in the current scope.
Generated code releases consumed operands after joining the result paths.

Regressions cover string concatenation/order, struct arithmetic/comparison/unary
methods, mixed-type errors, effects before failure and recovery afterward.
Compiled repeated owned addition/ordering also checks bounded root storage.
This is correctness and lifetime evidence, not a string allocation or speed win.
String/byte constants remain gated pending literal lifetime and local-helper
validation; wider owned arguments and specialized collection paths remain open.

## Integration gates still open

Generated Pop now releases owned operands; Dup and SSA local loads create distinct
roots. Peek-style SSA stores duplicate into the local and release its old root.
VM-mediated calls release callable/argument roots after marshaling ordinary Values to the VM,
and equality/logical operations release their consumed operands after evaluation.
Immediate paths branch around root helpers. Lazy duplication materializes the
current scope, and release cannot consume a caller's root.

Compiled bytecode tests exercise discard, local overwrite, dead-local release and callback patterns
at 8 and 128 repetitions: root-slot capacity remains at most four, no roots remain
after return, the VM stack is empty and returned-array identity is preserved.
Source regressions cover saved locals surviving unrelated discards and overwrites.
These checks do not establish lifetime coverage for every opcode, loop-entry/local
persistence path or legacy collection helper. Wider owned entry remains gated;
no collection speedup is claimed.

`ReleaseDeadLocal` now clears the paired SSA payload/kind, releases its owned root,
removes the named binding and invalidates cached dictionary pointers. Whole-function
contexts must leave the caller's VM slots untouched; only loop contexts clear
the physical slot. Direct-argument parameters have their own SSA binding/root.
Regressions cover copied-parameter native compilation, caller-slot isolation,
Null readback, live operand copies and bounded root storage. This does not widen
the native owned-argument entry guards.

1. Boundary/function lifetime hooks are connected. Extend their validation to
   real owned-value bytecode, callbacks and imported native execution while
   preserving the existing shared recursion budget.
2. Replace all named/local/constant/callable loads and legacy marker protocols with
   the same payload/kind representation. Root every owned operand independently.
3. Duplicate on operand duplication and local loads. Transfer or duplicate on
   stores according to peek/pop bytecode semantics. Release overwritten locals,
   consumed operands and dead locals. Account for SSA joins and loop iterations.
4. Decode/encode callback arguments and results without leaving hidden VM-stack
   values. Preserve reference VM operator, binding and error contracts.
5. Promote owned returns before callee cleanup; take boundary results before
   releasing boundary storage. Clean every failure exit without replaying effects.
6. Prove bounded live storage through actual native loops, imports, recursion,
   callbacks, failures and alias mutations. Passing the isolated store tests alone
   does not prove generated-code lifetime correctness.
7. Only then widen owned argument guards. Require
   the eight failure probes, both-runtime/JIT regressions and unchanged all50
   checksums, then paired optimized timings and production verification.

The isolated tests cover stale handles, signed payload transport, exhaustion,
cross-frame ownership, zero-copy promotion, COW snapshots, arbitrary unlink order,
repeated frame cleanup and bounded slot reuse. A deterministic mixed-operation
model checks nested frames and live roots independently. No speedup is claimed.

## Guarded collection rebinding

`TryAppendAssign` supports native array push/append and byte concat using the
same checked rebinding operations as the VM. The callable must still resolve to
the matching builtin; shadowed calls, changed destinations, incompatible types
and excessive nesting retain the evaluated operands for ordinary fallback.
Success consumes all three operand roots and skips the fallback call/store.
Unique storage is reused; retained snapshots and self-appends preserve COW.

`ReleaseRebindLocal` conditionally clears a matching mutable local snapshot before
an ordinary synchronous helper call. It preserves the already evaluated argument,
rejects native/async/generator callables, and retains snapshots whenever the VM
has a current exception handler. This check is conservative for native callees
whose VM caller has a handler. SSA roots, named locals and physical loop slots
use the same storage-identity condition.

`tests/jit_guarded_collections.rs` requires actual function and loop native entry
in addition to interpreter/VM/JIT results. Helper tests compare unique allocation
identity at 8 and 128 updates, verify COW snapshots, and bound native root storage.
These are correctness and allocation checks, not competitive speed evidence.

## Native iteration

`ForNext` consumes its iterable and index on both edges. A yielded value carries
its ordinary payload/kind pair; exhaustion carries no operand. The private helper
uses kind -4 only as an exhaustion signal, which lowering branches on before any
language-value use. A yielded null remains distinguishable from exhaustion.
Array elements are cloned as ordinary Values; bytes become nonnegative integers.
Iteration retains the language's snapshot semantics when the source is rebound.

Whole native functions step generators through the VM continuation API, preserving
lazy effects and errors. A locked native loop resumes at the original `ForNext`
before any generator step, so callbacks cannot reenter the locked environment.
Cold iteration callbacks still use the existing full VM bridge; compiled callbacks
can enter native code. Generator bodies are not promoted to native execution.

Loop-header stack depths now follow reachable edges. Return instructions terminate
analysis paths, and append/iteration success and fallback/exhaustion edges have
distinct stack effects. Unreachable increments after an early return cannot invent
loop-carried values. Regression coverage lives in `tests/jit_iteration.rs` and
bounded-root/error tests beside the native helpers.

Returns reached inside a compiled loop also resume the VM at the original return
instruction. This transfers the complete evaluated operand stack and persisted
locals before the VM performs function-frame cleanup. Both immediate and owned
return values are covered, including the pre-existing hot `while` return case.

VM callbacks may restore cloned call frames or relocate the global-scope vector.
After generator steps, ordinary function callbacks and overloaded operators,
`VMContext::refresh_vm_views` refreshes the VM-owned global-map and caller-slot
pointers on both success and error. It does not replace the native function's own
named-local map. Locked loops never take this path: they resume the VM before
callbacks. New callback helpers must preserve this pointer-refresh contract.

`vm_owned_views` records whether those pointers belong to the attached VM.
VM-created contexts set it; standalone contexts retain their supplied maps and
slots. Refreshing after callbacks must not silently replace external context
storage. The relocation regression forces a new scope-vector allocation and
checks frame replacement on generator success and failure without dereferencing
stale pointers.
If a VM-owned context uses the global map as its local map, both aliases refresh
together. Independently owned native locals remain untouched.
