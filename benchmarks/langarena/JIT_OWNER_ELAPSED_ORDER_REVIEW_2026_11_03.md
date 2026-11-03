# Worker-reuse elapsed review: order-confounded run

Run 41648504034, candidate 8a6df2590fb32bcd76d6d136de9c85d3289878dc,
baseline cd5f2b207a8f907d6c263f5a2e405412965344ca, artifact 13238671061.
API provenance, uploaded executable hashes, 192 observations (24 excluded
warmups), exact outputs, and recomputed ratios verified. Raw nonbinary evidence
is preserved in owner-elapsed-order-confounded-2026-11-03.json.gz; binaries remain
in /tmp/kujo-owner-elapsed-review/artifact.

## Correction to the initial status interpretation

The collector rotated twelve cells by round, then used the rotated position plus
round to choose variant order. Those two offsets canceled: each measured cell
kept the same variant first in all seven rounds. The original test checked only
that both orders occurred somewhere, and the verifier duplicated the defect.
Thus these are real observations, but not the claimed alternating-pair evidence.
Do not accept the candidate or attribute the observed slowdown from this run.

Observed JIT median baseline/candidate process ratios: direct 3.4870, parallel
4/8/16 workers 2.6970/2.3394/2.4718. VM direct 0.797619 (candidate 25.4% longer).
These remain hypotheses for corrected measurement, not accepted speed claims.

## Correction and verification

Choose variant order using round plus the cell's fixed identity; retain rotating
cell order. A behavioral test now requires opposite first variants on consecutive
measured rounds for every cell. It failed against the original collector on VM
direct and passes after the fix. The artifact verifier expects the corrected
schedule and includes a nonalternating-order negative control. Older artifacts
remain historical evidence but deliberately fail the corrected acceptance check.
No runtime or benchmark kernel changed. Rerun paired CI before deciding whether
the VM change is a runtime regression. Overall LangArena and clean-host results
remain pending.
