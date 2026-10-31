# Retained capture pool boundary: confirmed allocation gap

Accepted runtime `b1bf786` passes 72 allocation observations and 108 semantic
runs across VM, interpreter and JIT. This extends the existing retained-local
fixture at intermediate sizes and the compact-metadata boundary.

| Extra retained locals | Read bytes per added call | Write bytes per added call |
| --- | ---: | ---: |
| 2048 | 1266.0 | 1277.0 |
| 4096 | 1266.0 | 1277.0 |
| 6144 | 1266.0 | 1277.0 |
| 8190 | 2720287.0 | 2720298.0 |
| 8191 | 1650.0 | 1661.0 |
| 8192 | 1650.0 | 1661.0 |

VM/JIT allocation slopes agree. Slopes compare 16 and 32 calls. Counter/helper
bindings also contribute to captured membership; extra-local count is not exact
map length. Separate closure snapshots, aliases, writes and all retained values
pass the existing assertions. This is an allocation finding, not elapsed speed.

Source inspection finds different thresholds: compact metadata admission uses
scope name count, while frame recycling limits aggregate map capacity and key
bytes. A scope below compact admission can exceed recycling limits and recreate
its name/value tables on each call. The 8,190-extra-local control reproduces a
large repeated-call cost missed by the earlier 2,048/8,192 endpoints.

This remains an open section-3 fix target. Recheck against the next accepted
runtime before implementing. Cover key-byte and combined-scope limits, cold
captures, public host edits, immutable bindings, snapshots and bounded retention;
do not resolve it merely by increasing pool limits. The root dictionary candidate
is separate and unaccepted at this checkpoint. Sections 3/4 remain open.

[Evidence](capture-pool-boundary-2026-10-30.json) includes matching artifact
hashes, fixture sources, allocation rows, semantic statuses, independently
recomputed slopes, runner and verifier. No runtime changes accompany this report.
