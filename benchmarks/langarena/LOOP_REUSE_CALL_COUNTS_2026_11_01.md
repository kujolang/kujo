# Reusable native loops: verified VM-call reductions

With whole-function JIT disabled, six unchanged controls each scan 200 times.
Both pinned binaries preserve every expected result. Twelve measurements have
independently verified inventory, fixture hashes, report digests and counters.

| Shape | Width | Accepted fd7b764 VM calls | Candidate 348a6270 VM calls |
| --- | ---: | ---: | ---: |
| Condition call | 300 | 60,602 | 701 |
| Body call | 300 | 60,402 | 701 |
| Precomputed bound | 300 | 402 | 402 |
| Condition call | 3,000 | 600,602 | 701 |
| Body call | 3,000 | 600,402 | 701 |
| Precomputed bound | 3,000 | 402 | 402 |

Unlike the preceding one-entry loop candidate, the reusable candidate removes
repeated VM call dispatch across later scans. These counts establish structural
work reduction; they do not establish CPU-time percentages or elapsed speedups.
Measurements overlapped auxiliary compilation, and timing fields are deliberately
excluded from this comparison. Paired elapsed measurements remain pending.

Baseline binary SHA-256 is `45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`;
candidate SHA-256 is `f5393d6847fec1fb805731258cf57318aa8889a7997e4c60ed45b7d3de3eac11`.
The companion `loop-reuse-call-counts-2026-11-01.json.gz` contains 45
digest-verified fixtures, runner, manifests, raw outputs and measurement reports.
Candidate acceptance still requires the pending Mac test gate and cost review.
