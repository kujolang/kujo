# Paired collection replacement in captured arrays

Runtime e445f4a keeps two same-kind array or dictionary payload replacements
in bounded private storage until publication. Both writes reach snapshots,
callbacks and ordinary mutation boundaries in order. A third distinct index,
mixed payload type or scalar mutation falls back without dropping either write.
Negative indices and replacement-array assignment semantics remain unchanged.
Indices outside the private storage range use ordinary mutation; this introduces
no language index limit. Storage remains within the existing capture-journal size.

Requested bytes per added helper call, width 2,048, 16-to-32 calls, VM:

| Payload | Before | After |
| --- | ---: | ---: |
| array | 282962 | 4394 |
| dictionary | 283294 | 4726 |

All four new target groups (array/dictionary payloads, VM/JIT) are flat across
widths 3 and 2,048. All 18 previously flat root-array groups remain flat.
These are allocation measurements, not elapsed-time speedups.

Verified against accepted runtime f415211:

- 1256 library passes, 8 ignored.
- 410 integration passes, 1 ignored.
- All 150 test-size checksums across VM, interpreter and JIT.
- 1520 allocation comparisons and 2280 semantic checks, with no higher allocation totals or slopes: 288 paired-write, 432 root fallback, 480 historical struct and 320 nested comparisons.
- 15 additional scalar/compound contracts preserve dynamic operators, errors and selector ordering.
- 54 paired whole-process peak-memory observations, including two writes and retained snapshots, reviewed separately.
- Exact source/artifact hashes, test-name coverage, formatting and diff checks.

Memory assessment: All six large, unretained array/dictionary target groups reduced mean peak RSS by 4,296,704–4,497,408 bytes (9.78%–11.11%). Small and retained configurations were mixed: 13 groups had positive mean changes, up to 133,120 bytes; the highest mean percentage increase was 0.693%, and the largest individual increase was 368,640 bytes. These increases remain visible and are not established to be noise. Unchanged journal/activation layout, bounded two-handle storage, snapshot tests and non-regressing allocation matrices support accepting this specific optimization. Two local paired observations per group do not establish retained-heap bounds, leak absence, universal memory improvement or clean-host speed.

[Complete observations](array-collection-pairs-2026-10-29.json) preserve remaining
width costs. General multi-index updates, boxed-payload helper bodies,
compound/captured-scalar batching, capture materialization and representation
review remain open. Snapshot preservation can require copying; no necessary copy
was removed to improve the measurements. Sections 3/4 are not complete.
Production-size, platform and clean-host comparisons remain separate.
