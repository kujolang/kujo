# Loop cache reuse: Linux verification

Candidate `aae3919f6c161906b7dcdf8f4a8efcf44730180a` passed Linux run
41274756901, job 124754572467: **1,346 tests passed, eight ignored**.
This comprises 1,311 library tests and 35 tests across eight integration suites.
Artifact 13070749659 records the exact commit and matching source hashes.
Saved summaries and unique test membership were checked; the condition-call
suite matches all eleven tests declared in its frozen source.

The new tests verify chunk-scoped heat/block state, six native entries across
repeated calls to two functions, and three native entries across nested array
and integer-dictionary shapes while preserving input snapshots. The integration
helper executes VM, interpreter and JIT, disabling whole-function JIT to isolate
loop execution. Previously each fixture entered native code only once.

This is partial correctness evidence, not runtime acceptance or a speed claim.
Mac verification remains active. Follow-up commits 8064895b (statistics) and
17c3ccd4 (profiling invalidation) require their own checks, as does the final
combined candidate's 150-case matrix and paired measurements. Callback fallback
costs must also be measured to detect excessive native re-entry. Full compatibility
verification remains separate. Accepted runtime remains fd7b764.

The companion compressed JSON bundle contains six digest-verified artifacts.
The first draft collection fixture used numeric dictionary literal keys, rejected
by VM; it was corrected to supported dict() indexed writes. Only the corrected
fixture's all-runtime results support the reproduction claim.
