# Loop retry candidate: 150 verified test-size checksums

Frozen candidate `348a62700179e59beccaefddc7ed4da30992d67b` passes all fifty
unchanged LangArena test-size cases in each runtime: **VM 50/50, interpreter
50/50, JIT 50/50**. CLI SHA-256:
`f5393d6847fec1fb805731258cf57318aa8889a7997e4c60ed45b7d3de3eac11`.

Independent verification checks exact case order/configuration, runtime and
binary identity, zero exit codes, no truncated output, one exact expected
checksum line and one success summary per case. All benchmark source/config
hashes match the preceding verified eb299e0 matrix. Report digests, frozen
runtime source hashes and the copied binary digest also match.

The companion `loop-fallback-checksums-2026-11-01.json.gz` preserves eleven
digest-verified runner, manifest, report and log artifacts. Session 63174 completed
successfully. The recorded elapsed times overlap auxiliary compilation and are
not used for speed comparisons.

This is test-size correctness, not production coverage or candidate acceptance.
The CLI copy was made after its compiler completed while Mac auxiliary tests
were still building. The Mac test gate remains pending; verify the final Cargo
CLI digest matches this copy. Paired cost/timing review remains pending after
compilation. The accepted performance baseline remains `fd7b764`.
