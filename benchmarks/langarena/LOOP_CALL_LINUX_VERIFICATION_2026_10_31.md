# Loop-call candidate: Linux regression gate

Candidate `6d96c151e8b9f3d814043eebe5768ae5c61cde97` passed its
[independent release-mode Linux regression job](https://github.com/kujolang/kujo/actions/runs/41267049672/job/124731921815).
Exact source hashes, job identity, compiler artifacts, suite inventory and new
test membership were checked against the preserved Cargo log.

| Suite | Passed | Ignored |
|---|---:|---:|
| Library | 1,310 | 8 |
| Callback assignment | 2 | 0 |
| Callback depth | 1 | 0 |
| Callback promotion | 2 | 0 |
| Error propagation | 2 | 0 |
| Loop collection contracts | 15 | 0 |
| Loop condition calls | 8 | 0 |
| Loop field methods | 1 | 0 |
| Loop owned rebinding | 1 | 0 |
| **Total** | **1,342** | **8** |

All eight new condition-call tests executed, including the direct function-loop
compilation regression corrected after the [failed fixture round](JIT_FIXTURE_FAILURES_2026_10_31.md).
The runtime cases cover VM, interpreter and JIT outputs/errors, native loop entry
and completion, builtin aliases, rebinding, shadowing and callback effects.
The Linux SSG executable-start retry regression also passed in the library suite.

The [evidence bundle](loop-call-linux-verification-2026-10-31.json.gz) contains
six artifacts: run/job metadata, exact source/toolchain receipt, complete Cargo
log, checked test inventory and verifier source. Every payload was decoded,
compared with its original bytes and SHA-256 verified.

This completes one independent regression gate. Corrected local verification,
all 150 test-size checksums, before/after loop measurements, and the separate
compatibility/full-release gates remain pending. The accepted runtime remains
`fd7b764`; no speedup or overall candidate acceptance is claimed here.
