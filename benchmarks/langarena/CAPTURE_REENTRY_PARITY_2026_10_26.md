# Captured recursive writes — unresolved AST activation state loss

A returned closure updates its captured array from a recursive call, then reads
that same binding after the recursive call returns. VM and JIT-enabled execution
print the updated array on both calls. The interpreter prints the original array
on both calls. This control contains **no indexed assignment**, so it isolates a
capture-state problem from the separate root-after-index compiler decision.

The discrepancy reproduces on both the frozen `0d0f365` baseline and the corrected
native-default candidate. All six processes exit successfully; success means the
diagnostic completed, not that the outputs agree. Exact source, binary hashes,
commands and outputs are in `capture-reentry-parity-2026-10-26.json`. JIT-enabled
CLI output alone does not prove which instructions executed natively.

| Runtime | First call | Second call |
| --- | --- | --- |
| VM | `[9, 9, 9]` | `[9, 9, 9]` |
| Interpreter | `[1, 2, 3]` | `[1, 2, 3]` |
| JIT-enabled | `[9, 9, 9]` | `[9, 9, 9]` |

The specification's closure identity rule states that aliases and repeat calls of
the same closure share its cells; separately created closures keep independent
snapshots (`docs/LANGUAGE_SPEC.md`, closure capture rules). The interpreter's
recursive self-binding retains the same captured-environment `Arc`, but
`enter_captured_environment` clones its state for each activation. On return,
three synchronous call paths replace the captured environment with the completed
activation's environment. The outer activation can therefore restore and publish
its stale copy after the recursive activation wrote a newer value.

This is distinct from the historical missing-recursive-self-binding problem:
self-resolution succeeds here, but state updates are lost. A separate control in
which an index helper is a newly created closure preserves its own independent
snapshot on all three runtimes; that isolation must remain intact.

## Synchronous call routes and exception control

An additional 18 observations on the corrected candidate isolate five failures
and one passing control. Exact sources, outputs and binary identity are in
`capture-reentry-routes-2026-10-26.json`.

| Case | VM / JIT-enabled | Interpreter |
| --- | --- | --- |
| Direct recursive inner write | `9`, `9` | `1`, `1` |
| Outer write read by recursive inner call | `[9, 9]` | `[1, 9]` |
| Reentry through `map` callback | `9`, `9` | `1`, `1` |
| Reentry through indexed callable value | `9`, `9` | `1`, `1` |
| Inner write followed by caught exception | `9`, `9` | `1`, `1` |
| Distinct factory instances plus same-closure alias | `2`, `3`, `2`, `4` | `2`, `3`, `2`, `4` |

All processes exit zero; expected-output comparisons identify the failures. These
cases cover direct, computed-callee and native-callback interpreter invocation
routes. They establish that both entering and leaving a recursive activation
need correct shared-state visibility. The passing factory/alias control is an
invariant for the fix, not evidence that recursive capture state is correct.

## Expanded controls and imported runtime bridge

A further 36 correctness observations are pinned to runtime commit `8b4111b` in
`capture-reentry-bridge-controls-2026-10-26.json`, including complete source,
binary identity, output and exit status. The eleven ordinary controls pass in VM
and JIT-enabled execution. Interpreter execution fails nine state-reentry cases
and passes the distinct-factory/alias and immutable-binding controls. The added
failing routes include a global helper, generator, mixed global/captured helpers,
and an array whose earlier snapshot must remain unchanged.

A separate imported factory accepts a caller-defined relay and returns an AST
closure. The closure calls the relay, which reenters the same closure. The inner
call writes `9`; both outer calls incorrectly return `1` in **all three modes**.
In VM/JIT-enabled mode, the caller-defined relay crosses a bytecode callback
bridge. This establishes that an interpreter-instance-only activation stack is
not sufficient coverage for the fix. The test does not claim native JIT execution.

These are existing failures on the committed baseline, not regressions introduced
by the pending indexed-assignment change. All processes exit zero; output
comparisons detect the failures. No timings were collected from these controls.

## Required follow-up


- Preserve visibility and persistence of writes when the **same** captured closure
  is entered recursively or through aliases/callbacks.
- Keep separately created closures, factory instances and user collection copies
  isolated. Preserve immutable binding checks and caller-local scope boundaries.
- Cover all synchronous interpreter call routes, errors/unwinding, mutual or
  indirect reentry, reentry through separate VM callback bridges, and interaction
  with existing asynchronous write journals.
- Add regression tests on VM, interpreter and JIT; verify actual native execution
  where it is claimed. Measure environment/call costs before and after.
- Recheck captured-root indexed assignment after correcting capture reentry; do
  not infer the correct root from a stale interpreter activation.

No fix for this issue has been applied. It remains open in checklist section 3.
Native-default sharing is verified and committed separately; its checksum passes
do not close this gap.
