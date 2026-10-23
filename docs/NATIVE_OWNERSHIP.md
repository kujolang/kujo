# Experimental native owned-value lifetime mechanism

Status: **lifetime hooks and tagged VM-stack conversion connected; other owned operations pending**. Native invocation
wrappers and generated function entry/exit helpers now enter and leave lazy root
scopes. `src/native_owned.rs` is compiled with `runtime-jit`, but generated owned
loads and most operators do not yet use its codec. VM-stack pops now encode owned
callback results, and tagged pushes borrow roots when marshaling callback
arguments. This repairs the native `str(x)` return path from the original audit;
it does not establish full native parity/performance. Existing entry guards remain.

Conditions on tagged owned operands now borrow the root and use `Value::is_truthy`,
matching VM/interpreter truth rules without cloning the container. Immediate
conditions retain native evaluation. Stale roots report an error and generated
conditions exit through the existing error path. This covers callback-produced
empty strings and logical operations, but the original empty-global-string audit
still needs named-load conversion: its legacy integer marker has no owned tag.

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

## Integration gates still open

Tagged stack conversion alone is not complete operand lifetime management. Roots
currently survive until activation cleanup unless explicitly transferred. Generated
Pop, Dup, local overwrites and consumed arguments still need ownership-aware code
before long-running owned loops can claim storage bounded by live operands. Do not
use this partial integration to widen entry guards or claim a collection speedup.

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
7. Only then widen owned argument guards and enable `ReleaseDeadLocal`. Require
   the eight failure probes, both-runtime/JIT regressions and unchanged all50
   checksums, then paired optimized timings and production verification.

The isolated tests cover stale handles, signed payload transport, exhaustion,
cross-frame ownership, zero-copy promotion, COW snapshots, arbitrary unlink order,
repeated frame cleanup and bounded slot reuse. A deterministic mixed-operation
model checks nested frames and live roots independently. No speedup is claimed.
