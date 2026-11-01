# Performance-era unsafe contract review

Full gate `41268768044`, job `124738718845`, collected all test targets and
failed the unsafe inventory budget: 269 executable matches versus the recorded
99. The JIT-specific ceiling also remains 47 versus 147 current matches.
Neither limit has been increased. The difference is 100 matches in `src/jit.rs`,
15 in `src/vm.rs`, and 55 across ten integration-test files. Inventory matches
include declarations and test code; these are not 170 demonstrated defects.

The strict JIT safety checker initially found 103 boundaries missing its
canonical `SAFETY` / `Preconditions` / `Postconditions` schema. Many already have
informal safety comments. Completing a schema is not, by itself, a memory-safety
proof: each contract must agree with its implementation and callers.

## Reviewed ownership primitives

The first batch covers `refresh_vm_views`, `jit_duplicate_operand`,
`jit_release_operand`, `jit_release_dead_local`, `jit_take_parameter`,
`jit_load_literal`, and `jit_load_operand`. Review checked raw-pointer lifetime
and alignment requirements, immutable name metadata, exclusive mutation of
maps/slots/root stores, native activation ownership, and null/error handling.
The root codec validates handles instead of interpreting them as object pointers.
Literal loads clone compiler-retained values; parameter loads transfer ownership.
VM view refresh leaves loop-environment views and separately owned locals intact.

Only comments changed. Removing comment-only and blank lines produces identical
source before and after the batch (SHA-256
`ff147710224b9202820063185df2fceca773ae4f70bd3508933778087a95ec2f`).
`rustfmt --check` and `git diff --check` pass. The strict inventory generator
still classifies every row; generated locations were refreshed. The safety
checker still fails, now with 96 missing contracts across 145 checked boundaries.
No runtime rebuild was needed for this comment-only batch.

Remaining review includes the other JIT boundaries, VM pointer bridges,
test-only FFI/allocation instrumentation, and precise module budgets. The
existing inventory test must remain failing until that review supports an
explicit update; the current performance candidates remain unaccepted.
