# Exclusive code owner: two ARM boundaries verified

Exact source `cb931728f8cf7d59856f84830b4213a87eec2146`, workflow run
`41627350511`: Linux ARM64 job `125887927238` / artifact `13227328757` and
macOS ARM64 job `125887928322` / artifact `13228842173` independently pass
66 targeted tests each: six owner-lifetime tests and sixty integration tests
from ten pinned-source targets. Both include executable-entry revocation.

The artifact verifier checks source/host identity, exact test membership,
builds, summaries and hashes. macOS controls accept original and CRLF logs and
reject duplicated or fabricated integration membership. Linux controls were
also previously verified. This is 132 executions, not 132 distinct tests or
full-library acceptance. API job/run/artifact identities were checked separately.

The compressed `jit-owner-arm-boundaries-cb931728-2026-11-03.json.gz` preserves
raw logs, API provenance, receipts and verifier source with SHA-256 checked raw
bytes. The same run's Linux x64 minimal-feature build failed; this two-platform
result does not change that outcome. Intel macOS/Windows remained pending at
review. Later accounting/reset/pool commits require their own verification;
no task pooling, whole-candidate acceptance or speed result is established here.
