# Loop reuse integration candidate

Branch `codex/loop-reuse-integration-20261101` combines loop reuse, public
statistics and profile invalidation through `17c3ccd4` with compatibility fixes
through `2d3ee2ee`. The latter passed full workflow 41274970381 separately.
The merge completed without conflicts. Stripping comment-only/blank lines and
dead-code lint attributes confirms JIT and VM executable source still matches
17c3ccd4; the compatibility review did not replace the reuse implementation.

Regenerated unsafe and TODO inventories account for merged source line positions.
The executable boundary budget remains exactly 269, including JIT 147 and VM 15.
All three inventory contract tests and both affected artifact-freshness tests
pass when compiled directly against this worktree. Formatting, diff checks and
the JIT contract checker pass (145 boundaries, zero missing contracts).

These are preparation checks only. This combined revision has not passed its
runtime suite, checksum matrix or paired cost probes. Existing separate jobs
must be attributed to their exact commits, not this merge. Verify the final
combined CLI across VM/interpreter/JIT and all 150 test-size benchmarks, including
repeated native execution and callback fallback costs. No acceptance, production
coverage or competitive speed claim is established here.

Integration run 41276926940 exposed one CLI Clippy error: `should_compile`
became unused after VM loop admission moved to chunk-scoped state. Retain this
public legacy API and its existing unit tests with a narrow dead-code allowance,
matching the adjacent legacy inspection APIs. This changes no runtime behavior.
The original run remains evidence for 6551a57a only; the lint correction needs
its own combined gate before acceptance.

The corrected f1a59839 run 41277198605 passed Clippy, formatting and MSRV, but
parity job 124761681840 reproduced a JIT-disabled compilation failure: the shared
VM references three chunk-scoped admission methods absent from `jit_disabled.rs`.
Matching disabled stubs now always reject native loop admission. A regression in
the existing no-JIT parity suite runs repeated hot nested-array updates with JIT
requested both off and on, checking snapshots and zero compiled functions.
The fixture passes the pinned eb299e0 VM/interpreter as a syntax/semantics control;
that control does not validate the new stubs. The repaired no-JIT build and test
remain subject to the next immutable CI gate.
