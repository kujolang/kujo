# Production template JIT verification

Both template workloads pass unchanged production checksums on accepted runtime
`fd7b7646e8adf99d1a798302ef9a18978e4c330d` in JIT mode. Each uses count 50,000,
10 warmups and 50 measured iterations; both return checksum `3554506535`.
No runtime, input, algorithm or iteration-count change was made.

| Case | Total elapsed | Measured kernel interval | Peak RSS bytes |
| --- | ---: | ---: | ---: |
| Template::Regex | 148.641 s | 115.114 s | 118747136 |
| Template::Parse | 2127.594 s | 1749.792 s | 116584448 |

The parser completed in about 35 minutes 28 seconds. These are instrumented
single observations on the loaded local machine, not controlled performance
comparisons or speedup claims. Total time includes startup, setup, warmup and
checksum computation. Kernel time is the checkpoint interval from warmup-done
to run-50. RSS comes from the child runtime's `/usr/bin/time -l` output.

A three-second CPU sample occurred during parser warmup. Several subsecond
admission/semantic diagnostics and worktree preparation overlapped this run.
The sample cannot reliably attribute stripped Kujo functions. These effects and
changing user workload preclude treating this run as a clean speed measurement.

Independent verification passes exact case/configuration membership, pinned
sources and executable/wrapper identities, successful exits, untruncated output,
checksum/summary lines, complete monotonic checkpoints, measurement hashes and
RSS evidence. Native entries are present but do not prove native hot-loop
coverage. The separately reproduced [loop admission gap](TEMPLATE_LOOP_CONTROLS_2026_10_31.md)
remains open; an isolated candidate is not yet accepted.

`accepted-production-template-jit-2026-10-31.json.gz` preserves 30
digest-verified artifacts, including raw reports, counters, runner, verifier,
sample and diagnostic evidence. Accepted executable SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

Current verified production coverage is 22/150 runtime/case combinations.
Template VM and interpreter runs remain pending in the original sequence;
other production, clean-host and platform gates are not implied complete.
No Kostya contact occurred.
