# Active captured-local visibility: fix gate

Status: reproduced on runtime `2e020ee`; implementation pending. This does not
reopen the separate task-global isolation fix. No elapsed-time claim is involved.

`tests/ordered_task_capture_contracts.rs` checks both worker-to-caller and
caller-to-worker channel ordering for scalar, array, dictionary and nested-row
writes. Each case also checks the closure's eventual value and an independent
pre-mutation snapshot. Expected output is `9, 9, 1` on separate lines. Channels
establish ordering; the 120-second process bound only prevents a deadlock from
hanging the test runner.

Current result: VM and JIT each pass all eight cases. Interpreter fails all eight
with `1, 9, 1`, exit zero and empty stderr. The first read is stale; eventual
publication and independent snapshots are correct. The interpreter test is
explicitly ignored in the default suite **only while this known defect is open**;
run it with `--include-ignored` as the mandatory failing fix gate. It must be
unignored when the runtime is repaired. A default green run does not close this
item. The complete current execution, including the ignored test, failed as
expected. See `ordered-capture-gate-2026-10-26.json` for hashes and raw output.

## Storage and synchronization paths requiring the fix

- `Interpreter::enter_captured_environment` clones the owner or active snapshot;
  the activation then reads and mutates its own `Environment.scopes` maps.
- `Environment::get`, `set`, `assign_checked`, `mutate`, `mutate_checked` and
  `try_mutate_checked` are the binding access paths. A correct fix must make
  channel-ordered reads observe completed writes through every relevant path.
- `leave_captured_environment` and VM bridge publication synchronize at call
  boundaries. Captured-write journals prevent unrelated completed writes from
  overwriting one another, but do not provide live visibility during a call.
- `capture_for_callable`, `visible_bindings`, `capture_prefix_snapshot`,
  `restore_capture_locals` and generator snapshots read maps directly. They must
  materialize current values if active storage becomes shared. Separately
  created closures must still snapshot, not alias, their captured values.
- `enter_global_function` suspends local scopes and reuses their indices for
  helper parameters. Shared capture ownership must move with the suspended
  locals; indexing alone cannot identify a binding. `swap_local_scopes` and
  generator resume have the same ownership constraint.
- `release_rebound_local` and the initial-alias optimization in `eval_stmts`
  mutate map slots directly. Ownership optimizations must never clear another
  activation's live binding or bypass the new storage protocol.
- Task-global provenance and execution ownership remain distinct. The fix must
  not turn submission snapshots into shared globals or introduce lock cycles
  through callback owners, `vm_globals` and task capture context.

A channel-specific refresh or synchronization only at submission/await would
leave other access paths wrong. Prefer a general captured-binding storage
protocol, with explicit snapshot creation and suspension behavior. Avoid a
whole-environment copy before every expression or native call: that would restore
the scaling problems this work is intended to eliminate.

## Acceptance beyond this first reproducer

Retain these 24 checks and add active helper/generator suspension, new closure
snapshot creation after an ordered update, caught-error writes, shadowed helper
parameters, distinct-binding overlapping tasks and indexed/field mutation cases.
Check callback reentry and owner cleanup/lifetimes without holding a shared lock
across user code. Preserve immutable binding errors and rejected mutation
behavior. Same-binding concurrent read-modify-write remains non-atomic.

Run task/global/module/callback integration suites, library checks and all 150
test-size checksums against the final candidate. Use allocation/scaling controls
for costs; clean-host elapsed comparisons remain pending per user direction.
