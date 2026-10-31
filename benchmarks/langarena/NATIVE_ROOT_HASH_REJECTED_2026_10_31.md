# Native root hash candidate: rejected helper tradeoff

The hash-only candidate is not accepted. Against accepted runtime `fd7b764`,
all 48 scalar-read external pairs were slower. The twelve shape medians were
7.4–31.3% slower, including cached reads. At 2,048 live roots, owned-row
lifecycles improved 64.3–71.3%. That improvement does not justify the scalar
regression. Loaded-host timings are diagnostic, but this repeated regression
is not dismissed as noise.

Correctness passed: 1,310 library tests, 43 targeted integration tests and
150 test-size checksums across VM, interpreter and JIT; eight existing library
tests remained ignored. Four alternating executable pairs produced 576 helper
observations and 57.6 million outer indexed reads. Value, kind, error and
reference-count assertions passed. Three internal samples were summarized
before comparing each external pair. Whole-workload timing was not started
after this rejection.

The candidate only changed root-table hashing and added an equality-scan
regression test. Issued handles, exhaustion, stale-handle validation and frame
ownership rules were unchanged. Frozen candidate CLI SHA-256:
`998be922bf6b7b488c63f9cea095f5714c533a9dc60f92b0865bd482f304cb52`.
Frozen library SHA-256:
`88da5e7692010a5f748e530500d502de34f50d589bc62b8a2d659687a6fc33f5`.

Symbolized helper disassembly shows additional conditional selection around
the borrowed-array path in this build. This is a follow-up hypothesis, not
proof of the entire slowdown's cause. A separate revision directly borrows the
owned array before generic fallback and adds validation-order coverage. Its
tests and measurements are pending; these passes do not certify that revision.

Raw paired observations, verification receipts, source hashes, rejected patch,
runner scripts and disassembly are preserved with per-artifact SHA-256 digests
in `native-root-hash-rejected-2026-10-31.json.gz`. Local originals are in
`.audit-evidence/native-root-hash-fix-2026-10-31/`.

This work belongs to section 5. The bounded sections 3/4 review cutoff remains
complete. Clean-host competitive and production/platform gates remain open.
