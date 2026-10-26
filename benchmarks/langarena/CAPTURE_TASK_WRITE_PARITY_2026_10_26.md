# Captured task-write publication gaps

Status update: ordinary global-helper journal scope is fixed in `15a225d`; see
[accepted verification](TASK_HELPER_JOURNAL_VERIFICATION_2026_10_26.md).
The other task ownership gaps below remain open. Original reproductions are retained.

Two language tasks invoke aliases of one ordinary captured function. One writes
`a = 1`, the other writes `b = 1`. A shared counter and channel hold both calls
until both have written; then the caller joins both tasks and reads `a + b`.
Expected output is `2`. VM and JIT-enabled execution produce `2`; the interpreter
produces `1` on both baseline runtime `3a9cbe4` and the uncommitted synchronous
reentry candidate. All six executions exit zero without a timeout.

Exact source, binary hashes, candidate source hashes and outputs are retained in
`capture-task-write-parity-2026-10-26.json`. These are correctness observations,
not timing or native-JIT execution evidence. The candidate fixes tested recursive
reentry but does not close this concurrent publication gap.

In `src/interpreter/tasks.rs`, `execute_task_body` tracks and merges captured
writes for `AsyncFunction`. An ordinary function instead goes through
`call_user_function`; its captured invocation clones an environment and publishes
the completed environment as a whole. Two independent activations can therefore
overwrite each other's changes to different bindings. The new activation stack
and VM-bridge refresh address nested calls, not two simultaneous task workers.

The language specification states that captured AST writes to different bindings
do not overwrite each other. This finding concerns separate bindings and is not
a request to make same-binding read-modify-write atomic. Preserve independent
closure snapshots, same-closure alias identity, cancellation and errors when
extending write-set handling to ordinary task callbacks.

This is related to the recursive capture-state bug, but remains independently
reproducible after the current reentry candidate. No fix is claimed here.

## Async helper parameters can masquerade as captured writes

A second fixture uses the existing async write journal. Task zero writes captured
`a = 1`; task one writes captured `b = 1` and calls a global helper
`func shadow(a) { return a }`. Separate channel gates force task zero to commit
before task one. Expected final sum is `2`; interpreter execution prints `1` on
both baseline and candidate. VM/JIT-enabled print `2`. Renaming only the helper
parameter to `unrelated` makes all six baseline/candidate runs print `2`.
These twelve additional bounded observations are in the same JSON evidence file.

`Environment::enter_global_function` suspends caller locals but retains the
capture journal and its original scope limit. The helper parameter now occupies
scope index 1, which previously held captured locals. Defining helper parameter
`a` records `(1, a)` as a capture write. After caller scopes are restored, merging
that journal publishes the old captured `a` value over the other task's update.
The name-only control and explicit commit ordering distinguish this from a
same-binding atomicity race.

Task-write handling therefore needs both ordinary-callback coverage and journals
that follow their actual binding scopes through helper/generator suspension.
Simply enabling the current journal on more call sites is insufficient. The
current reentry candidate leaves this existing async helper case unresolved.

Three additional baseline controls cover a captured write after the helper
returns, a nested global helper whose parameter shadows the captured binding,
and a caught helper exception followed by a captured write. Each prints `2` in
VM/JIT-enabled mode and `1` in the interpreter, with successful process exits
and no timeouts. Their nine observations, sources and baseline binary hash are
stored under `async_helper_scope_controls` in the JSON evidence. The eventual
fix must restore journal scope after both normal and exceptional helper returns;
excluding helper parameters alone must not discard subsequent captured writes.

## Captured async calls retain stale task globals

A separate sequential control removes concurrency and helper calls. A factory
returns an async closure with one captured local counter. Its first call writes
a global to `9` and increments the local counter. The submitting parent then
sets its global to `42` before the second call. Section 5.7 of the language spec
requires task globals to be submission snapshots while retaining the closure's
local state: expected results are `[9, 1]`, `[42, 1]`, then parent global `42`.
VM/JIT-enabled match. The interpreter instead returns `[9, 1]` on the second
call, retaining the previous task's global. Parent global and captured local
controls both pass. All six baseline/V4 candidate executions exit successfully.

`execute_task_body` replaces the worker's submission environment with the whole
captured environment, including its old globals. Its journal also merges global
writes back into that capture. Correcting task-local publication must distinguish
submission globals from persistent captured locals; fixing only name/scope
journal pollution cannot satisfy this control. Exact source, binary and candidate
source hashes are recorded under `task_global_submission_snapshot`. This is a
pre-existing mismatch, not a regression introduced by the V4 reentry candidate.

## Task/caller reentry still reads stale active captures

Two coordinated controls on accepted synchronous-reentry runtime `45360b9`
expose a remaining task boundary. In the first, an active captured function
writes `data = 9`, spawns an alias that reads the same capture, and awaits it.
In the second, the task writes `data = 9` and its awaiting caller then reads the
capture. Both should print `9`, followed by `9` from a later call. VM/JIT-enabled
do so; the interpreter prints `1`, then `9` in both controls. All six processes
exit successfully without timeouts. Exact sources and the binary hash are under
`task_caller_reentry_visibility` in the evidence JSON.

These are ordered visibility checks, not concurrent read-modify-write tests.
The synchronous activation stack and VM-bridge publication do not synchronize
an active caller with a separate task worker. Merging different-binding writes
at return alone cannot correct the stale read before the active call returns.
Task ownership work must cover both publication and visibility at task/await
boundaries. The helper-parameter journal patch under verification is separate.

## Ordinary task submission globals also remain stale

The stale-global case also reproduces through `spawn_task` of an ordinary
captured function on accepted runtime `15a225d`. The first task writes global 9
and captured local 1. The parent changes its global to 42 before submitting the
second task. VM/JIT return `[42, 1]`; the interpreter returns `[9, 1]`. The parent
still reads 42 in every mode. All three runs exit successfully without a timeout.

Exact source, binary hash and outputs are recorded under
`ordinary_task_global_submission_snapshot` in the accompanying JSON. This extends
the existing task-global defect to ordinary callbacks; it is not a new timing
claim. Refreshing globals only in the async task entry path would leave this
case unfixed. Imported module lexical globals must remain distinct from caller
globals when correcting both paths.
