# Experimental native owned-value lifetime mechanism

Status: **named operands, callbacks and truthiness connected; ownership integration incomplete**. Native invocation
wrappers and generated function entry/exit helpers now enter and leave lazy root
scopes. `src/native_owned.rs` is compiled with `runtime-jit`. Ordinary named loads
now use tagged roots, preserve local/global/builtin precedence and leave no hidden
values on the VM stack. Calls explicitly marshal their callable before arguments,
and checked named assignments borrow the tagged value. Specialized collection
helpers still retain legacy paths. VM-stack pops encode owned
callback results, and tagged pushes borrow roots when marshaling callback
arguments. This repairs the native `str(x)` return path from the original audit;
it does not establish full native parity/performance. Existing entry guards remain.

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

The native execution census shows that owned-argument call boundaries are heavily
exercised. The existing native path uses incompatible VM-stack markers and an
append-only object vector. Completing a common lifetime mechanism is a prerequisite
to replacing those paths; widening entry guards first would expose incorrect values.

## Storage and ownership

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

## Empty dictionary literals

The supported `MakeDict(0)` path converts its constructed dictionary from the
helper stack into a tagged owned operand immediately. It no longer returns an
integer sentinel or leaves a hidden dictionary on the VM stack. Local stores
marshal the tagged value even when its payload is integer `-1`; slot-only local
names are included in native name tables for helper lookup. Direct-mode local
loads use the same owned codec.

Regression coverage includes returns, locals, callback arguments, truth/equality,
reassignment to `-1`, saved snapshots, recursive returns and independent subsequent
mutation. Repeated compiled literal creation/discard joins the bounded-root tests.
Nonempty literal compilation and specialized indexing helper protocols remain
separate gates; this change does not widen their supported opcode set.

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
