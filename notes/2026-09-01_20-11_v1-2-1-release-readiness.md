# Kujo Field Notes — v1.2.1 Release Readiness

**Date:** 2026-09-01
**Session:** 20:11 local
**Branch/Commit:** main / 0246233d2d2911f235d643b7c6980c02e9968e54
**Scope:** Kujo v1.2.1 routed-HTTP boundary correction, release guardrails, and downstream RAG security verification.

---

## What I Changed

- Prepared the routed-HTTP request-body correction and peer-socket support as the
  v1.2.1 patch release.
- Added a tag/source/npm version-consistency guard to the binary release workflow.
- Made the release workflow check out the exact pinned RAG security fixture and
  execute its live raw-socket request-boundary regression with the candidate
  interpreter and VM binaries.
- Refreshed the generated unsafe and v1 code inventories after the runtime changes.

## Gotchas (Read This Next Time)

- Kujo v1.2.0's earlier body regression did not prove that a complete declared
  body returned without waiting for the read deadline. The strengthened RAG
  fixture uses a keep-alive raw client and exact `Content-Length`; it fails on
  the old runtime and passes with this candidate.
- The normalized Cargo package still replaces the repository-local patched
  `tiny_http` with upstream 0.12.0, which has no
  `Server::http_with_read_timeout`. Crates.io publication remains blocked.
- A local full ecosystem gate must use the exact fixture revisions pinned by the
  release workflow. Nearby checkouts are useful diagnostics but cannot replace
  the hosted immutable-fixture result.

## Things I Learned

- The release gate needs both exact fixture revisions and a behavioral
  regression that distinguishes the previous artifact from the candidate.
- A tag/source/npm equality check is necessary because archive names alone do
  not prove which runtime version they contain.

## Debug Notes (Only if applicable)

## Verification Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Source/tag/npm version guard | PASS | Accepted `v1.2.1`; rejected an incorrect `v1.2.0` tag. |
| Rust formatting, Clippy, HTTP unit tests, private-spool tests | PASS | Candidate source at `0246233`; HTTP body tests 3/3 and focused private-spool security tests 2/2. |
| npm workspace tests and package dry runs | PASS | 12/12 tests; all six npm packages packed successfully. |
| RAG live request boundary | PASS | Exact-body keep-alive and oversized-body checks passed with the candidate in interpreter and VM modes. |
| Generated inventory freshness | PASS | 3/3 freshness checks passed after regeneration. |
| Hosted release state | PASS | Run `33574098795`. |
| Hosted artifact validation matrix | PASS | Run `33574098819`. |
| Hosted LSP contract matrix | PASS | Run `33574098806`. |
| Hosted release gate | PASS | Run `33574098786`, attempt 2, passed with the CI fixture matrix; the tag workflow separately pins the strengthened RAG fixture at `b850db11f353a9ffbaea627d1991e62d7c281fb0`. |
| Hosted field-notes and tool-artifact guards | PASS | Runs `33574098880` and `33574098826`. |
| `cargo publish --dry-run --locked` | BLOCKED | The normalized package cannot compile the hardened HTTP calls against upstream `tiny_http` 0.12.0. |

Warnings and exceptions:

- GitHub release binaries are the canonical distribution for v1.2.1. Do not
  publish the crate until its normalized dependency graph preserves the hardened
  HTTP API.
- The repository-required `UNBLOCK_V1_RELEASE` directive was supplied in the
  active release-execution context.
- Hosted release-gate attempt 1 encountered a nondeterministic failure in the
  existing SSG subprocess error-reporting test. An unchanged failed-job rerun
  passed all 814 binary tests and the complete gate; no release code was changed
  to suppress or bypass the test.
- This note authorizes no synthetic threat-model approval. RAG's accountable
  human review and deployment attestations remain separate release evidence.

Release readiness sign-off:

The v1.2.1 source is ready for a signed GitHub release tag at the commit adding
this note. Publication is complete only after all platform assets, per-asset and
consolidated checksums, npm publication (when configured), and the
published-artifact smoke workflow have succeeded. Crates.io is explicitly
excluded by the documented dependency-normalization blocker.

## Follow-ups / TODO (For Future Agents)

- Create and verify the signed v1.2.1 tag after the note commit's hosted checks pass.
- Verify every published platform artifact and checksum, then promote stable
  release strings and downstream immutable pins.
- Resolve the patched `tiny_http` packaging boundary before enabling crates.io.

## Links / References

- Release process: `docs/RELEASE_PROCESS.md`
- Candidate gate: `scripts/release_candidate_gate.sh`
- Binary release workflow: `.github/workflows/release-binaries.yml`
- RAG regression fixture: `b850db11f353a9ffbaea627d1991e62d7c281fb0`
