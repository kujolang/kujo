# Overlapping synchronous task callbacks lose distinct captured writes

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
