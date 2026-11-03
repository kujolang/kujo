# Bounded owner storage and Intel boundary evidence

Two distinct prerequisite revisions now have independently verified artifacts.
They are not a single latest-candidate platform matrix.

| Scope | Exact source | Run / job / artifact | Targeted passes |
| --- | --- | --- | ---: |
| Linux x64 bounded idle storage | `0a0dd452ba3bfe7bb5beecd9a4573bc994ac9c40` | `41633091423` / `125907347574` / `13231755517` | 73 |
| macOS Intel owner transfer boundary | `cb931728f8cf7d59856f84830b4213a87eec2146` | `41627350511` / `125887927225` / `13231426706` | 66 |

Linux coverage is seven owner tests, sixty integrations, two minimal-feature tests
and four accounting/reset tests. The pool test checks exclusive cross-worker
checkout, pointer retention and execution, four quota rejection paths, policy
mismatch, poisoned-lock behavior and returned-value survival. Task-route reuse,
later recovery tests and physical-resource behavior are outside that revision.

macOS Intel coverage is six owner tests and sixty integrations, including native
entry revocation. Together with the previously verified two ARM and Windows
artifacts, it completes review of those four non-Linux-x64 jobs on `cb931728`.
That run's Linux x64 minimal-feature build remains failed. The corrected Linux
feature/accounting evidence has its own later source identity; do not report the
original five-platform workflow as green.

For both artifacts, exact source/host, test membership, build/summary outcomes and
API run/job/artifact identities were checked. Six Linux and four macOS verifier
controls passed. The raw logs, API provenance, receipts and verifiers are retained
in `jit-owner-pool-and-intel-gates-2026-11-03.json.gz`, with every raw-byte SHA-256
verified after encoding. No full-library, latest-platform, all150, production,
physical-memory or speed claim follows this targeted prerequisite evidence.
