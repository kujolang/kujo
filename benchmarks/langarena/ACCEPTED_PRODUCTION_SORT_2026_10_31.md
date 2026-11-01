# Accepted-runtime production sorting verification

QuickSort and MergeSort pass unchanged production inputs in JIT mode on accepted
runtime `fd7b7646e8adf99d1a798302ef9a18978e4c330d`. This closes two current production
checksum cases, not the all-50 or all-runtime gate. No runtime change was needed.

Both cases use 400,000 elements, 10 warmup iterations and 50 measured iterations.
The expected checksum is `61757772` for each. Inputs, algorithms, warmups and
iteration counts were not reduced. Each case ran sequentially in an isolated
process with a one-hour deadline; both exited successfully without timeout.

| Case | Checksum | Total elapsed | Measured kernel interval | Peak RSS |
| --- | --- | ---: | ---: | ---: |
| Sort::Quick | PASS, 61757772 | 227.892 s | 192.410 s | 133,959,680 bytes (127.8 MiB) |
| Sort::Merge | PASS, 61757772 | 984.314 s | 821.644 s | 387,084,288 bytes (369.2 MiB) |

These are single instrumented observations on the loaded local macOS machine.
They establish completion and resource observations, not controlled speedups,
competitive scores or a universal memory bound. Total elapsed includes startup,
preparation, warmup and verification. The kernel interval is the raw checkpoint
difference from `warmup-done` to `run-50`. Peak RSS comes from `/usr/bin/time -l`
around the child runtime, not the parent runner.

## Verification

The independent verifier checks exact case membership, pinned input hashes,
binary/wrapper identity, successful exits, untruncated output, exact checksum and
summary lines, the complete expected checkpoint sequence, monotonic timestamps,
runtime-measurement hashes and peak-RSS evidence. All checks pass.

Runtime counters record 59,107,908 native function/boundary entries for QuickSort
and 72,399,888 for MergeSort. These confirm native execution, not native CPU-time
coverage. VM fallback remains supported and is not excluded by those counts.

Frozen executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.
Unchanged `run.json` SHA-256:
`8fe7baed41e8eb0a233057c4406446029f7e5d792cf3a3ac6fc7e714964a155f`.

`accepted-production-sort-2026-10-31.json.gz` preserves raw reports, streamed output,
measurement files, host metadata, source hashes, the timed wrapper, runner and
independent verifier with per-artifact SHA-256 digests. Local originals remain in
`.audit-evidence/accepted-production-sort-2026-10-31/`.

The other 48 JIT production cases and production VM/interpreter verification on
this accepted revision remain open. Earlier passes retain their earlier runtime
identities. Future runtime changes require new final-candidate evidence.
Clean-host comparisons, other platforms, maintainer review and release remain
separate pending gates. No Kostya contact occurred.
