# Windows exclusive-owner boundary verified

Run `41627350511`, job `125887927426`, artifact `13230280157`, exact source
`cb931728f8cf7d59856f84830b4213a87eec2146`: independently verified **65 passes**,
comprising five owner-lifetime tests and sixty callback/task/generator integration
tests from ten pinned-source targets. Source/host, exact membership, summaries,
build results and API run/job/artifact identities were checked.

Four verifier controls accept original and CRLF logs and reject duplicate and
fabricated integration membership. The compressed
`jit-owner-windows-cb931728-2026-11-03.json.gz` preserves raw logs, API provenance,
receipts and verifier sources, with raw-byte SHA-256 verification after encoding.

This covers returned values, disabling JIT, cross-worker owner movement, current
globals/embedded literals and current cancellation tokens. Windows does not run
the executable-entry revocation test: the pinned backend's Windows reclamation
limitation remains open. This is compatibility evidence, not memory reclamation.

Later accounting/reset/pool/task-route changes are outside this revision's scope.
They still need their own test and resource gates. This result does not certify
all-platform latest-candidate acceptance, all150 LangArena or speed improvement.
