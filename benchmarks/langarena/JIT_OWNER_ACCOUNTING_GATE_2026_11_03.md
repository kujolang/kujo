# Corrected minimal-feature and owner accounting gate verified

Run `41630118425`, job `125897239486`, artifact `13229612432`, exact source
`fc56848eedc627a294ba98c127219bf86ae4f29d`: all targeted steps completed
successfully and the downloaded artifact independently verifies **71 passes**:

- Six optimized private-owner lifetime tests.
- Sixty callback/task/generator integration tests from ten pinned targets.
- Two no-default-features tests: returned-value/owner lifetime and the explicit
  disabled-network diagnostic contract.
- Three accounting tests: all four compilation definition paths, loop reuse and
  bytecode identity, and visible metadata/literal retention accounting.

Exact source/toolchain, API run/job/artifact identities, test membership, summaries,
build results and raw-byte hashes were checked. Six verifier controls accept
original/CRLF logs and reject duplicate/fabricated integration names plus fabricated
accounting and no-JIT test names. The prior Linux minimal-feature failures remain
historical failures; this is the verified result after the feature-guard correction.

`jit-owner-accounting-fc56848e-2026-11-03.json.gz` preserves the raw logs, API
provenance, receipts and verifier sources with per-file SHA-256 hashes.

This is targeted evidence for `fc56848e`, not the later idle reset, bounded pool,
task-route or callback/mapper changes. Their runtime gates remain pending. It is
not full-library, all-150 LangArena, production or performance acceptance. No
new speed claim or main-branch integration follows this result alone.
