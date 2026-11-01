# Accepted-runtime production JSON verification

All three JSON benchmarks pass unchanged production inputs in JIT mode on
`fd7b7646e8adf99d1a798302ef9a18978e4c330d`. Each uses 20 warmups and 100 measured iterations.
No runtime code, benchmark algorithm, input size or iteration count changed.

| Case | Coordinates | Verified checksum | Total elapsed | Peak RSS bytes |
| --- | ---: | ---: | ---: | ---: |
| Json::Generate | 20000 | 120 | 7.591 s | 73170944 |
| Json::ParseDom | 10000 | 3989922952 | 14.608 s | 134864896 |
| Json::ParseMapping | 25000 | 3826684672 | 56.609 s | 228868096 |

These are single instrumented observations on a loaded local macOS machine,
not clean-host comparisons or speedup claims. Total time includes setup and
warmup. RSS is the child runtime's `/usr/bin/time -l` observation.

The independent verifier passed exact case/configuration identity, unchanged
source hashes, accepted executable and wrapper hashes, successful exits,
untruncated output, checksum/summary lines, complete monotonic checkpoints,
measurement identity and native-entry evidence. Native entries do not establish
native CPU-time coverage. The accepted executable SHA-256 is
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.

`accepted-production-json-2026-10-31.json.gz` preserves 23 digest-verified
artifacts including raw results, measurements, host metadata, runner and verifier.
Local originals remain in `.audit-evidence/accepted-production-json-2026-10-31/`.

Together with the sorting evidence, five JIT production cases are verified on
this accepted runtime. The other 145 runtime/case combinations remain open;
prior results on other revisions are not substituted. Clean-host and platform
gates remain pending. No Kostya contact occurred.
