# Fallback retry: exact local structural results

The CLI compiler for frozen revision `348a6270` finished while Cargo continued
building auxiliary test executables. A separate copy of the completed CLI was
pinned at SHA-256 `f5393d6847fec1fb805731258cf57318aa8889a7997e4c60ed45b7d3de3eac11`.
Its Mac test gate is still pending; the exact revision already passed all Linux
CI gates. Cross-check this hash against Cargo's final CLI after local completion.

| Unchanged control | Before native entries | After native entries | After native completions |
| --- | ---: | ---: | ---: |
| Three callback-condition scans | 801 | 11 | 0 |
| Callback followed by builtin rebinding | 503 | 11 | 2 |

Both controls pass VM, interpreter and JIT with exact callback effects and `PASS`
output. Their hashes match the fixtures preserved in the before-evidence bundle.
These are work counts collected during compilation, not timing improvements.
The fallback path remains correct, and later eligible calls recover native
execution. Timing comparisons wait until compilation finishes.

The unchanged 150-case checksum run has started using this frozen copy. Its
ignored generated `port.kujo` was restored to the worktree with the exact hash
from the preceding verified matrix. No benchmark input or implementation changed.
Local runtime tests, checksum completion and paired timing/cost review remain
required before acceptance; `fd7b764` remains the accepted baseline.

The companion `loop-fallback-after-2026-11-01.json.gz` preserves fourteen
digest-verified manifest/report/stdout/stderr artifacts. The manifest explicitly
labels the pending Mac test gate.
