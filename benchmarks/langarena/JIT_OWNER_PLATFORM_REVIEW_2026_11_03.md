# Worker reuse: all five platform artifacts reviewed

Source `d5c8ad0770d07641933d47ffde1b6e9271023927`, workflow run
`41640183947`. All five terminal successful jobs now have independently reviewed
raw artifacts. This completes this revision's targeted five-platform gate, not
whole-candidate acceptance.

| Platform | Artifact | Verified targeted passes |
| --- | ---: | ---: |
| Linux x64 | 13234118043 | 81 |
| Linux ARM64 | 13232944462 | 75 |
| macOS ARM64 | 13235145710 | 75 |
| macOS x64 | 13236566867 | 75 |
| Windows x64 | 13234986517 | 74 |

The 380 passes cover exact pinned owner-test membership and all ten selected
integration targets (62 integration tests per platform). Linux x64 additionally
covers two no-JIT tests and four compiler-accounting tests. Windows excludes
executable-entry revocation and does not establish Windows memory reclamation.

Source revision, target host, build results, test membership and summaries were
verified with `probes/verify-vm-platform-lifetime.rb`. API run, job and artifact
metadata establish the source/run association and each completed job's success.
Twenty-two verifier controls accept original/CRLF evidence and reject fabricated
or duplicate membership. These are targeted semantic checks, not full-platform
library suites or speed measurements.

`jit-owner-platform-review-2026-11-03.json.gz` preserves the API metadata, raw
targeted logs, verification receipts and controls with SHA-256 hashes. Every
archived content hash was checked after writing. Linux economics/resource
payloads are deliberately outside this review's scope.

That first archive preserves the original four-platform review. The final
`jit-owner-platform-final-2026-11-03.json.gz` adds Intel macOS raw logs, verification
and four controls plus terminal run/job/artifact API metadata. All five jobs and
the overall run report success; the additional artifact's run and source identity
match exactly. Later revisions add test-only diagnostics; this gate certifies
the pinned runtime source above, not execution of those later diagnostics.

Candidate acceptance still requires review of concurrent/repeated memory
measurements, paired uninstrumented timing, and
selected production checks. No candidate integration or release follows from
these platform results alone.
