# Worker reuse: four platform artifacts reviewed

Source `d5c8ad0770d07641933d47ffde1b6e9271023927`, workflow run
`41640183947`. Four terminal successful jobs have independently reviewed raw
artifacts. Intel macOS remains pending; this is not a completed five-platform gate.

| Platform | Artifact | Verified targeted passes |
| --- | ---: | ---: |
| Linux x64 | 13234118043 | 81 |
| Linux ARM64 | 13232944462 | 75 |
| macOS ARM64 | 13235145710 | 75 |
| Windows x64 | 13234986517 | 74 |

The 305 passes cover exact pinned owner-test membership and all ten selected
integration targets (62 integration tests per platform). Linux x64 additionally
covers two no-JIT tests and four compiler-accounting tests. Windows excludes
executable-entry revocation and does not establish Windows memory reclamation.

Source revision, target host, build results, test membership and summaries were
verified with `probes/verify-vm-platform-lifetime.rb`. API run, job and artifact
metadata establish the source/run association and each completed job's success.
Eighteen verifier controls accept original/CRLF evidence and reject fabricated
or duplicate membership. These are targeted semantic checks, not full-platform
library suites or speed measurements.

`jit-owner-platform-review-2026-11-03.json.gz` preserves the API metadata, raw
targeted logs, verification receipts and controls with SHA-256 hashes. Every
archived content hash was checked after writing. Linux economics/resource
payloads are deliberately outside this review's scope.

Candidate acceptance still requires the remaining platform review, expanded and
concurrent/repeated memory measurements, paired uninstrumented timing, and
selected production checks. No candidate integration or release follows from
these four results alone.
