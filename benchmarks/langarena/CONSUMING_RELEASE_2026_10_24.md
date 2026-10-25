# Consumed-call operand release verification — 2026-10-24

Runtime `4e42bd2`, built from checkout `0c58ee9`, compared with frozen
`aa30071`. The exact binary identities, source/input hashes, raw checksum
reports, timing samples and memory observations are retained in
[the machine-readable audit](consuming-release-2026-10-24.json).

All 50 unchanged test-size cases passed in each of VM, interpreter and JIT:
150/150 exact registry/config/checksum/exit checks. JIT fallback is allowed;
this result does not establish native execution coverage.

| Runtime | Before median | After median | Median paired speedup | Favorable pairs |
| --- | ---: | ---: | ---: | ---: |
| VM | 7.979637 s | 7.642037 s | 1.019153× | 4/7 |
| JIT | 1.982125 s | 1.904624 s | 1.036301× | 7/7 |

These are whole-process QuickSort test-size measurements. CPU speed limits
changed from 55% to 71%; there was no competing compiler or profiler from this
verification pipeline. Shared-host load remains uncontrolled. The modest JIT
improvement is directional evidence, not clean-host or competitive acceptance.
The VM result is not evidence of a direct benefit from this JIT-only change.

| Peak RSS | Before, two trials | After, two trials |
| --- | --- | --- |
| VM | 43.22, 50.32 MiB | 51.64, 51.72 MiB |
| JIT | 49.07, 50.05 MiB | 49.01, 49.95 MiB |

Two samples are descriptive only and do not establish a memory improvement
or regression. The separate allocation probe in
[CONSUMING_CALLS_2026_10_24.md](CONSUMING_CALLS_2026_10_24.md) demonstrates
reduced allocation traffic; cumulative allocated bytes are not peak RSS.

Full production coverage, original-baseline/Goose/Rust comparison on a clean
host, supported-platform verification and broader performance gates remain
open. A separate sequential production run uses this frozen candidate and
frozen production inputs, with per-case deadlines and peak-memory capture.
