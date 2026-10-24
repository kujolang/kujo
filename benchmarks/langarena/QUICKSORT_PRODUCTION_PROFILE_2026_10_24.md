# Production QuickSort CPU diagnostic — 2026-10-24

The preceding audited release `aa30071` passes unchanged production JIT
QuickSort: 400,000 elements, 50 measured iterations, checksum **61757772**,
exit 0 and `Summary: 1, 1, 0`. The driver recorded 301.13 seconds for the process,
including preparation and warmup. Compilation of the next candidate and a CPU
sampler overlapped this run, so elapsed time is diagnostic only.

A ten-second, five-millisecond-interval macOS sample ran during warmup after
preparation. Its executable UUID matches the frozen release, and the binary hash
matches the earlier release audit. The raw sample is preserved as a losslessly
compressed artifact; hashes and full checksum output are in
`quicksort-production-profile-2026-10-24.json`.

Visible top-of-stack counts include memory movement (206) and environment lookup
(134), alongside idle waits (3,112) and unresolved Kujo addresses. These counts
include multiple threads and are not CPU percentages. The executable is stripped
and no matching symbol map was available; internal runtime attribution remains
unresolved. This does not establish a new runtime bottleneck or justify changing
value semantics or environment visibility.

The next profile should retain symbols or a linker map for the exact measured
binary. Reproduce and isolate a proposed cost before optimizing it. This single
production result does not certify the latest `4e42bd2` runtime, the full suite,
VM/interpreter production coverage, competitive speed, or supported platforms.
