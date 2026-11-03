# Worker reuse: Linux release and all150 correctness verified

Two independently reviewed terminal gates now cover the worker-reuse runtime.
Runtime source, Cargo files, canonical port fragments and test configuration are
identical between the two revisions below. Their test/workflow changes remain
separately identified. This closes these correctness checks, not candidate
acceptance or competitive performance.

| Gate | Source | Run / artifact | Verified outcome |
| --- | --- | --- | --- |
| Linux release | `d5c8ad0770d07641933d47ffde1b6e9271023927` | `41640188422` / `13233743005` | Seven jobs successful; 1,333 library + 39 selected integration passes |
| All150 test-size | `1ff97dcfe83c5f1c9350ef5ecb1106e027a12f0c` | `41641594358` / `13233613930` | Exact fifty checksums each in VM, interpreter and JIT |

## Linux gate

All seven jobs pass: formatting, MSRV, minimal release smoke, Clippy, VM/interpreter
parity, release gate and optimized library/loop regressions. The independently
reviewed library/integration artifact contains 1,372 passing tests and nine
explicit ignores. Eight are existing manual profiling tests; the ninth is the
isolated owner-resource diagnostic executed separately in the resource workflow.
No ignored test is counted as passed.

All thirteen owner tests from the pinned source appear exactly once, including
numeric argument-kind changes, bytecode edits, cancellation, capabilities,
unwind, eviction and owner transfers. All eight selected integration targets
have exact source-derived membership; the library runs with zero filtered tests.
Source/host hashes, build success, summaries, API provenance and complete job logs
are retained. Four verifier controls accept the original evidence and reject
fabricated owner/ignored names or duplicated integration membership.

## All150 gate

Job `125936200047` runs the explicitly selected checksum regression; the unrelated
lifetime matrix is skipped as intended. All three runtime sections contain the
exact fifty name/checksum lines in original configuration order and exactly one
successful summary. Registry membership, source identity and build records verify.

All 915 recorded source/input file hashes were checked against Git blobs at the
pinned commit. The uploaded frozen CLI bytes independently hash to:

`343964c7c36d5dce4b12f83bcd848c0de989a12bac7a2b41f2c3c9893deae8e4`

This executable identity is an actual off-host byte verification, unlike the
resource artifacts which carry only a CI executable-hash receipt. Source/binary
post-run rechecks also match. Seven verifier controls accept the original artifact
and reject duplicate checksum membership, altered checksum, wrong runtime,
missing source, changed binary or failed build evidence.

The test runs all fifty cases in one process per runtime. It is correctness
coverage for unchanged test-size inputs, not an isolated per-workload timing
comparison or proof that every hot region executed natively.

## Remaining acceptance

The later 4,096-task resource extension is test-only and pending execution; it is
not certified by these source revisions. Full platform artifact review, repeated
and concurrent retention, paired uninstrumented elapsed measurements and selected
production acceptance remain open. No release, integration or speed multiplier
is authorized by these results alone.

`jit-owner-combined-gates-2026-11-03.json.gz` preserves raw logs, provenance,
verification receipts, controls and verifiers. Executable bytes remain in the
local downloaded artifact and GitHub artifact, not duplicated into Git history.
