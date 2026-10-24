# Bounded native callback argument buffers — 2026-10-24

Runtime `1174b7b`, compared with `9f401eb`. Raw allocation logs and verification
identities are in `native-argument-pool-2026-10-24.json`.

Native bytecode callbacks can acquire an empty argument vector from a VM-local
pool instead of allocating a fresh vector for every stack suffix. The standard
compiled callback path returns the vector after arguments have been consumed or
the invocation has unwound. Immediate/named argument setup releases the vector
after publishing the argument values into the call context or local map.

Each active call exclusively owns its buffer. The pool holds at most 32 empty
vectors, each with capacity at most 256 `Value` elements. Oversized buffers are
dropped when argument consumption finishes. Retained capacity is a bounded
memory tradeoff, not a claim of reduced peak resident memory.

Positional staging keeps declaration order and moves consumed slots to null.
Unconsumed values are cleared on unwind in their original order. No live value
is retained in the pool. The native helper borrows only the disjoint pool field
while draining the VM stack. Existing malformed-stack handling is preserved.

Builtins and interpreter callables currently consume their vectors, so pool
acquisition is restricted to bytecode callables. An initial candidate let
builtins consume pooled capacity; the tighter byte budget caught that additional
traffic. The final predicate and a regression preserve reusable buffers across
builtin calls. Callable resolution, capability checks and fallback dispatch are
unchanged.

## Structural QuickSort measurement

Default-feature debug library, macOS x86_64, 8,192-element JIT cases:

| Input | Requests before → after | Requested bytes before → after |
| --- | ---: | ---: |
| Unique | 43,902 → 21,954 | 9,044,408 → 1,084,600 |
| Shared | 43,898 → 21,950 | 10,156,728 → 2,196,920 |

Unique-input requests decrease approximately 50%, and cumulative requested bytes
decrease approximately 88%. Unique backing is retained without array-sized
copies; shared input detaches once and preserves its saved snapshot. Full sorted
outputs and ownership checks pass at sizes 128, 1,024 and 8,192 with VM and JIT.

The optional `--expect-argument-pool-budget` probe gate requires fewer than
25,000 requests and 2,500,000 cumulative requested bytes for both 8,192-element
JIT cases. The final candidate passes. The pre-admission candidate fails the
byte gate. These diagnostic budgets are not wall-time or peak-memory gates.

## Verification and remaining work

- 1,094 library tests pass; 8 intentionally ignored.
- New unit coverage checks argument order, partial initialization, unwind,
  buffer identity reuse, active-buffer isolation, released values, pool bounds,
  oversized buffers and preservation of pooled capacity across builtin calls.
- 44 targeted integrations pass, including VM/interpreter/JIT return values,
  snapshots, helper ownership, recursion, errors and native stack behavior.
- No-default-feature compilation, formatting and diff checks pass.

The histogram still includes one-element builtin argument vectors and small
callable-name allocations. Release timing, the new runtime's all-50 checksum
matrix and peak-memory measurements remain pending. Prior frozen-release
acceptance applies to `93e7caa`; it does not automatically cover this change or
the intervening storage-identity fix. Production, clean-server comparison and
supported-platform gates remain open.
