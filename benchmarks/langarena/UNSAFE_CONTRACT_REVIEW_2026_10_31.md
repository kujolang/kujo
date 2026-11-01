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

## Field and indexed-replacement contracts

The second batch covers `jit_field_read`, `jit_field_write`,
`jit_index_set_result`, and `jit_prepare_field_call`. Review traced compiler
retention of field-name `Arc<String>` allocations, generated operand release,
canonical VM field/index operations, integer-key caching, and checked receiver
suffix reordering. Contracts distinguish cloned inputs from consumed root slots,
including errors after consumption, and require valid exclusive VM access for
key caching. This does not replace the outstanding VM bridge review.

Only comments changed; the same stripped-source hash above remains identical.
Formatting, diff checks and strict inventory generation pass. The safety checker
still exits 2, with **92 missing contracts across 145 boundaries**. Inventory
budgets remain unchanged; no runtime rebuild or full CI rerun is warranted for
these comments alone. Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch2.log`.

## Read, error and return contracts

The third batch covers `jit_indexed_read`, `jit_indexed_read_cached`,
`jit_loop_indexed_read`, `jit_scalar_type_error`, `jit_arithmetic_error`,
`jit_runtime_failed`, `jit_set_return_scalar`, and `jit_return_scalar_kind`.
Review checked preserved input roots and independently rooted read results,
physical loop-slot lifetimes, cache handle/liveness checks, generated 16-byte
cache initialization with 8-byte alignment, immediate scalar decoding, checked
arithmetic diagnostics, and cancellation-pointer retention by task ownership.
Return helpers record/read tags without transferring owned roots.

Executable source remains identical under the same comment-stripping check;
formatting, diff checks and strict inventory generation pass. The safety checker
still exits 2, now with **84 missing contracts across 145 boundaries**. This is
partial contract review, not runtime acceptance. Budgets remain unchanged and
no runtime rebuild was needed. Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch3.log`.

## Root inspection and stack transfer contracts

The fourth batch covers `jit_owned_truth`, `jit_owned_equal`,
`jit_stack_push_scalar`, and `jit_stack_take_scalar`. Review followed validated
root lookup, immediate decoding, equality/truth inspection, current-frame
ownership checks, and generated consuming Call versus borrowing/peek marshaling.
Contracts distinguish a cloned operand whose root survives from a transferred
operand whose root must no longer be used, and require the VM stack allocation
to be disjoint from context/root storage.

The stripped-source hash remains unchanged. Formatting, diff checks and strict
inventory generation pass. The safety checker still exits 2 with **80 missing
contracts across 145 boundaries**; no budget change or runtime acceptance follows.
Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch4.log`.

## Loop continuation, locals and native entry/exit contracts

The fifth batch covers `jit_loop_resume_guard`, `jit_loop_resume_prepare`,
`jit_load_local_slot`, `jit_store_local_slot`, `jit_enter_function`, and
`jit_leave_function`, plus four unsafe calls in the cancellation/entry-balance
regression. Review checked the generated loop stack reconstruction protocol,
bounds-checked physical slot access, native entry rollback, matched generated
exits, owned return promotion, and caller-retained cancellation/name lifetimes.
The tests keep context and atomic flag live, verify that failed entries acquire
no depth, and pair the successful entry with exactly one leave.

Comments alone changed, preserving the stripped-source hash above. Formatting,
diff checks and strict inventory generation pass. The safety checker still exits
2, now with **70 missing contracts across 145 boundaries**. VM bridges and
remaining boundaries still require review; budgets and acceptance are unchanged.
Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch5.log`.

## Ownership regression call-site contracts

The sixth batch reviews fourteen unsafe test boundaries: dead-local removal;
live/retired truth inspection; consuming transfers with reused handles, invalid
kinds, outer-frame ownership and missing stacks; immediate bit preservation;
snapshot-preserving stack marshaling; and three recursive owned-lifetime probes.
Test-owned containers remain live, helper access is exclusive, and assertions
inspect them only between calls. Deliberately invalid codec identities exercise
checked errors rather than invalid raw pointers. Recursive probes preserve the
entry/leave pairing and promote results through their enclosing invocation.

This is comment-only review: the stripped-source hash remains unchanged.
Formatting, diff checks and strict inventory generation pass. The safety checker
still exits 2 with **56 missing contracts across 145 boundaries**. No runtime
rebuild was needed, and no budget or acceptance criterion changed. Evidence:
local `.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch6.log`.

## Retained metadata, append access and iteration

The seventh batch reviews ten boundaries: retained struct/key metadata reads,
the dictionary key Arc increment/from-raw pair, append target/result access,
`jit_for_next`, and four iteration regression call sites. Review traced compiler
metadata retention and generated pointer emission; checked append binding lookup;
generator continuation restoration and VM-view refresh; and item, exhaustion,
byte, invalid-index and invalid-iterable cases. Generator callbacks require
exclusive VM access without a held environment lock; physical loop execution
resumes before those callbacks. The broader VM bridge review remains outstanding.

Only comments changed and the stripped-source hash remains identical. Formatting,
diff checks and strict inventory generation pass. The safety checker still exits
2 with **46 missing contracts across 145 boundaries**; budgets remain unchanged.
No runtime rebuild or acceptance claim follows. Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch7.log`.

## Indexed mutation, callback and binding contracts

The eighth batch reviews twelve boundaries: `jit_indexed_write`,
`jit_owned_operator`, `jit_local_initialization`, three indexed-write tests,
parameter-transfer/load mutation, and four named-load/shadowing test sites.
Review traced operand cloning before destination borrowing, mutability checks,
VM indexed-write behavior, callback view refresh, physical initialization flags,
and named-parameter ownership transfer. Test map edits occur between helper
calls; retained pointers are compared rather than dereferenced after mutation.

Comments alone changed; the stripped-source hash remains identical. Formatting,
diff checks and strict inventory generation pass. The safety checker still exits
2 with **34 missing contracts across 145 boundaries**. Budgets and runtime
acceptance remain unchanged. Evidence: local
`.audit-evidence/ci-lint-compatibility-2026-10-31/jit-safety-contracts-batch8.log`.
