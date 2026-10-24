# Builtin argument-buffer reuse — 2026-10-24

Runtime `aa30071`. Compared with runtime `1174b7b`, builtin dispatch now keeps the argument vector
in an outer wrapper, borrows it for synchronous dispatch, and recycles it after
success or error. Early-return method handlers and higher-order callbacks use
the same wrapper. Active nested invocations own separate buffers. JIT calls may
now acquire reusable buffers for native builtins as well as bytecode functions.
Interpreter callables retain their existing transfer path.

The existing limit remains 32 empty vectors, each at most 256 Value slots.
Arguments are cleared before recycling; live language values are never cached.
Capability checks, receiver handling, callback dispatch and value semantics stay
in the existing dispatch body. This is bounded retained capacity, not proof of
lower peak memory.

## Measurement

The unchanged QuickSort allocation probe verifies every sorted element and
saved snapshot at 128, 1,024 and 8,192 elements, VM and JIT. Debug-library JIT
results for 8,192 elements:

| Input | Allocation requests before → after | Requested bytes before → after |
| --- | ---: | ---: |
| Unique | 21,954 → 14,638 | 1,084,600 → 89,624 |
| Shared | 21,950 → 14,634 | 2,196,920 → 1,201,944 |

Unique backing is retained with zero array-sized copies. Shared input detaches
once and preserves the snapshot. The optional `--expect-builtin-pool-budget`
requires fewer than 16,000 requests and 1,300,000 requested bytes for both JIT
cases; both pass, along with prior budgets. The preceding runtime's recorded
counts exceed these limits. Raw counts and histogram are stored in
`builtin-argument-pool-2026-10-24.json`.

A separate allocator regression verifies zero allocations for 128 warmed
`to_int` builtin bridge calls, excluding construction of input stacks and names.
Unit tests verify buffer identity reuse and released arguments on success/error.
A new VM/interpreter/JIT test checks nested builtin callbacks, unchanged snapshots,
error recovery and actual native compilation of the conversion helper.

## Validation and limits

1,095 library tests pass (8 intentionally ignored), and 125 targeted integration
tests pass, including ownership, callback promotion, native environments and
capability boundaries. Formatting and no-default-feature compilation pass. This is allocation evidence only; latest-candidate release timing,
all-50 checksum coverage, peak memory, production and supported-platform gates
remain open. Small callable-name allocations remain visible in the histogram.
