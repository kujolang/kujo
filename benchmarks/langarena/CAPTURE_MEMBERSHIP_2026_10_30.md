# Exact captured-name validation without redundant hashes

Runtime `18ebd21` compares oversized captured binding names in their original
iteration order. On the first difference it validates the remaining names with
ordinary lookup. Equal counts alone never authorize reuse: host replacement,
reordering, removal, reinsertion and immutable binding rules remain protected.
There are no new persistent fields or allocation paths.

The component experiment covers 27 cases in five alternating rounds. Unchanged
8,192/16,384-name sets need zero hash lookups instead of one per name; reordered
sets retain ordinary lookup. Local timing observations remain diagnostic. The
matching 100,000-call workload passes; its symbolized sample now shows name
comparisons in capture activation where the baseline repeatedly sampled lookup.
This does not establish a whole-program speedup or competitive performance.

Verification against `7724558`:

- 80 target allocation comparisons: all unchanged; all 32 added-call slope groups
  have no regression. All 120 VM/interpreter/JIT semantic checks pass.
- 36 cold-capture allocation pairs and 144 split-owner pairs, with 270 runtime
  checks: no measured allocation regressions. Exact matrices, sources and artifact
  identities independently verified.
- 1,264 library tests pass, eight ignored; 414 integration tests pass, one ignored.
  Test names/statuses match the baseline plus the new exact-membership regression.
- All 150 test-size benchmark checksums pass across VM, interpreter and JIT.
- Formatting, diff and frozen source/artifact checks pass. Seven deliberately
  corrupted target reports are rejected by the independent verifier.

All 24 initial peak-RSS pairs remain in the evidence. The largest initial mean
increase was 6,985,728 bytes (14.372%) for 8,192 retained locals, VM write; both
observations increased. Six additional interleaved comparisons ranged from
−5,193,728 to +2,785,280 bytes. Six baseline-versus-baseline controls ranged from
−12,156,928 to +6,524,928 bytes. The increase did not repeat consistently; its cause
is not established. Unchanged allocation totals and unchanged storage layout do
not prove identical RSS. These local observations are retained as uncertainty,
not dismissed or represented as a memory improvement.

The required exact name scan remains linear: public host maps can change without
an invalidation hook. Broader capture materialization/fallback, unsupported
collection paths (including general selectors and scalar-first journals), and
representation review remain open. Sections 3/4 are not complete. Production-size,
platform and clean-host competitive acceptance remain separate.

[Complete evidence](capture-membership-2026-10-30.json) includes identities,
fixtures, component observations, runtime tests, all checksum results, raw memory
measurements, verifier implementations and the acceptance receipt.
