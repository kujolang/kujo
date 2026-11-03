# Task code reuse: independently reviewed Linux prerequisites

Three completed Linux artifacts now pass source-pinned verification. These are
successive revisions, not independent full-suite runs or final-candidate acceptance.

| Revision | Run / job / artifact | Targeted passes |
| --- | --- | ---: |
| `ff3409eceeb6dcbd6c52fe27d8fd80cc5b340515` | `41633864624` / `125909970412` / `13231761265` | 74 |
| `5ae0d6b4741139f071736c71f8bd863bb205c8e1` | `41634268128` / `125911348918` / `13232016350` | 75 |
| `a7dd800155316b0fd2559846b01901fe0631fef6` | `41635703828` / `125916224158` / `13231987816` | 78 |

Initial task integration covers eight owner tests, sixty integration tests, two
minimal-feature tests and four accounting/reset tests. The bridge revision adds
one integration test for serial async work through parallel-map/callback routes.
The last revision adds capability and cancellation/error/rebinding owner tests,
plus explicit overlapping-worker integration coverage: ten owner tests and
sixty-two integrations, with the same six minimal/accounting tests.

The tests require fresh task globals and policy, retained native execution,
flat compilation for the tested sequential reuse paths, independent overlapping
workers and retained snapshots. Exact test names are derived from each pinned
source. Linux executable-entry revocation is included. Other library tests are
filtered; these counts must not be added as unique tests.

For every artifact, GitHub run/head/job/artifact provenance, exact source/host,
build outcomes, test membership and raw summaries were independently verified.
Six verifier controls per artifact accept original/CRLF logs and reject duplicate
or fabricated integration, accounting and minimal-feature membership. The raw
logs, API metadata, verifier receipts and control outcomes are preserved in
`jit-owner-task-route-gates-2026-11-03.json.gz`; all 33 embedded file hashes were
verified after decoding the archive.

Later unwind/eviction tests, forty-cell economics, isolated resource measurements,
final-source broader regression/platform/all150 checks and elapsed comparisons
remain open. No worker-reuse candidate is accepted or merged by this review.
