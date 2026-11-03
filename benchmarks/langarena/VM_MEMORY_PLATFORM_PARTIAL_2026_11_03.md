# Memory candidate: three platform lifetime gates verified

This is a partial platform result, not final candidate acceptance. In workflow
`41613864999`, exact source `baaf23931e413474528e43f4cad0559bff75945b` passes:

| Platform | Job | Verified tests | Executable revocation tested |
| --- | ---: | ---: | --- |
| Linux x64 | 125842289961 | 63 | Yes |
| Linux ARM64 | 125842289692 | 63 | Yes |
| macOS ARM64 | 125842289351 | 63 | Yes |

Each platform runs three named code-owner lifetime tests and 60 tests across ten
generator/callback/task integration binaries. The verifier checks exact owner-test
names, ten Cargo test targets, successful build result, all ten integration
summaries, zero integration failures/ignores/filtering, passing-test counts, source
commit, target host and raw hashes. Other library tests are deliberately filtered;
these are not three full-library runs. A negative control changes the reclamation
test's success to `ignored` while leaving the summary unchanged; verification
rejects it.

The Linux/Mach mapping test checks executable permissions after VM destruction
without dereferencing retired entries. Its child-process isolation prevents other
parallel tests from reusing code pages during the observation. The runtime prefix
and JIT implementation remain unchanged from candidate `75445306`; `baaf2393`
adds Linux test coverage and workflow configuration. Source diff and identities
are preserved with the evidence.

At this capture, macOS Intel and the corrected Windows-only run remain pending.
The original Windows job in this workflow failed formatting before runtime tests;
subsequent retries and build-environment repairs are retained separately. Windows
lifecycle success, when obtained, will not prove executable reclamation: the
pinned backend lacks its Windows deallocation implementation. No platform's
targeted pass establishes universal memory behavior or an elapsed-speed win.

`probes/verify-vm-platform-lifetime.rb ARTIFACT_DIRECTORY PLATFORM EXPECTED_COMMIT`
rechecks downloaded artifacts. Workflow/job provenance must also match the recorded
API results; a local source receipt alone is insufficient. The adjacent
`vm-memory-platform-partial-2026-11-03.json.gz` retains raw three-platform logs,
source/toolchain receipts, API snapshots, verifier, negative control and source
comparison with verified per-artifact hashes.

## Exact integration membership follow-up

Acceptance review reproduced a weakness in the original verifier: replacing the
generator-after-owner-destruction pass with a duplicate existing test left sixty
passing lines and unchanged summaries, so the old verifier accepted it. This is
a verifier defect, not evidence that the actual platform runs omitted the test.

The verifier now compares all sixty passing names against the ten test files at
the exact tested Git commit, retaining their source hashes and per-file names.
Its discovery is intentionally limited to these reviewed top-level, directly
annotated test functions; it is not general Rust test discovery. The tested
commit must be available in the local Git object database. Cargo target, build,
summary and workflow-provenance checks still apply.

All three real platform artifacts pass the stronger check. Regression controls
accept unchanged and CRLF-normalized logs and reject both a duplicate test and a
fabricated replacement while leaving the passing counts unchanged. Run
`probes/test-vm-platform-membership.rb ARTIFACT_DIRECTORY PLATFORM EXPECTED_COMMIT`
to repeat these controls. The separate
`vm-memory-platform-membership-2026-11-03.json.gz` preserves the reproduced old
acceptance, corrected verification receipts, pinned test sources, original logs
and regression scripts. The earlier evidence archive remains immutable.

Both remaining CI jobs have now completed their private-owner test step and are
running the integration step. Final macOS Intel/Windows artifacts and the overall
memory-candidate acceptance decision remain pending.
