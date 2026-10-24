# Frozen collection/argument-pool release verification — 2026-10-24

Runtime `aa30071`, built from checkout `13d0448`, compared with frozen `93e7caa`.
The candidate includes root storage identity, bytecode argument buffers and builtin
argument buffers. Structural reports isolate those fixes; this timing comparison
measures their combined effect. It does not compare with the original baseline,
Goose or Rust.

All **150/150 unchanged test-size checksums** pass: 50 distinct pinned cases in
VM, interpreter and JIT. The audit checks the exact registry/configuration,
exit status, checksum output and summary. JIT fallback remains allowed; passing
checksums does not establish native kernel coverage. Binary and runtime source
identities, configuration hashes and raw outputs are retained in
`builtin-pool-release-2026-10-24.json`.

## Paired QuickSort timing

Seven rotating rounds, 28 total samples. No agent-started compilation overlapped
timing or memory measurement. Workload sizes, iteration counts and warmups remain
unchanged; measured-loop and total-process times are retained separately.

| Runtime | Before median | After median | Median paired speedup | Favorable pairs |
| --- | ---: | ---: | ---: | ---: |
| VM | 7.9715 s | 7.7653 s | 1.0265× | 4/7 |
| JIT | 2.1430 s | 2.0718 s | 1.0454× | 6/7 |

The reported CPU speed limit changed from 100% to 71%. Individual paired ratios
vary considerably, including slower candidate observations. These small local
improvements are provisional, not controlled-host acceptance or proof that
allocation reductions produce proportional elapsed-time gains.

## Peak memory

Two checksum-verified process samples per runtime/variant, maximum resident size:

| Runtime | Before MiB | After MiB |
| --- | --- | --- |
| VM | 51.86, 51.73 | 47.44, 51.82 |
| JIT | 50.39, 50.65 | 48.88, 48.51 |

These samples are descriptive and do not establish a general peak-memory
reduction. Cumulative requested allocation bytes are a different metric.

## Remaining gates

Production workloads, original-baseline/Goose/Rust controlled comparison,
supported-platform verification and repeatable regression/competitive gates
remain open. Callable marshalling still clones names; the separate consuming
helper candidate has not been integrated or tested. This report certifies only
the named frozen candidate and measured scopes, not future source changes.
