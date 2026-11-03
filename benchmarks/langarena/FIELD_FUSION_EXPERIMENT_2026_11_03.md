# Guarded indexed scalar field experiment

Status: implemented on `codex/native-field-fusion-20261103`; **not accepted**.
Baseline `c27e3a8b449a406f118a63e35a75e580d7c3c856` retains only the
previously verified local-store runtime change. The separate scalar-write and
worker-code-reuse experiments are not included. Accepted branch is untouched.

## Hypothesis and falsification

The independently reconstructed old-executable priority profile (CI41725794191,
artifact13275765423; report/archive on implementation commit10d3be0e) shows Neural
root encoding and Value cloning. A supplemental allocator-ancestor inspection
finds 29 exclusive allocator samples under Value::clone/indexed_read. This is a
sampling clue, not a current-runtime time fraction or a count of allocations.

Adjacent whole-function `IndexGet; FieldGet` materializes an intermediate struct,
including its name and owned root, even when the requested field is immediate.
The candidate borrows the array/struct only for one helper invocation and returns
an immediate scalar. It retains no pointer or borrowed view across helper calls.
Owned fields, methods, ArgParser precedence, missing fields, non-array inputs and
invalid indices retain the old checked helper sequence and intermediate cleanup.
No loop-region fusion or incoming edge to the second opcode is allowed.

Reject the candidate if unchanged NeuralNet does not show repeatable elapsed
improvement, if another workload has an unexplained regression, or if any semantic
or lifetime check fails. Lower root/allocation counts alone do not establish speed.

## Verification and identity

Runtime/test commit `ad7b3fd1`; first gate source `52e2e26942e5c3573004295d08460bee02015821`.
CI41730777183 runs full library tests, selected integrations, all150 exact checksums
and 16-workload alternating VM/JIT comparisons against c27e with matching flags.
It freezes binary/source hashes, toolchain, build logs, outputs and raw timings.
CI remains pending; no candidate speed or correctness result is claimed here.

Generated-code unit controls compare scalar kinds/exact floating-point bits,
owned-array backing, negative/out-of-range indices, error text, prior snapshots,
root cleanup and issued-root counts against an unfused branch-barrier variant.
The all-runtime fixture covers mixed types, returned collection mutation,
dictionary fallback, method receivers and errors. Instrumented JIT assertions
require both fast and fallback paths to execute. Additive counters are documented
in RUNTIME_MEASUREMENTS.md; they count helper calls, not CPU/instruction coverage.

Local `cargo fmt --check` and 11 verifier negative/positive controls pass. The new
fixture passes on the frozen accepted Mac CLI in VM/interpreter/JIT; these are
fixture compatibility controls, not tests of the newly compiled candidate.
Local full builds are deferred to CI because free disk is approximately1GiB.

Remaining before acceptance: inspect candidate test/checksum evidence, independently
verify raw pairing/identities, inspect actual Neural and other workload results,
confirm in an independent session if promising, and run complete Neural state /
float / snapshot and allocation/resource checks using the frozen pair. No benchmark
kernel or algorithm was edited. No release, accepted-branch merge or maintainer contact.
