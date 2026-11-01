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
