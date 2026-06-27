# Kujo v1.0 Final Review Blockers (2026-06-20)

Status: active final-phase handoff before human v1.0 readiness review

Superseded operational checklist: `docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md`.

## Current Boundary

Kujo is closer to final review after the 2026-06-20 security and dependency
advisory passes, but it is not ready for release publication until the
release-flight item below is completed with real evidence or explicitly
deferred by release owners.

## Completed In This Pass

- Added `--deny-private-net` so strict outbound destination policy can be
  forced from the CLI without relying on environment variables.
- Aligned `async_http_get(...)` and `async_http_post(...)` with synchronous
  HTTP guardrails: URL scheme validation, destination policy, timeout, and
  response-size limits.
- Capped `parallel_http(...)` batches at `128` URLs and rejected non-string
  entries instead of silently dropping invalid inputs.
- Escaped raw HTML in `render_markdown(...)` and native SSG HTML helpers, and
  replaced unsafe Markdown link/image schemes with `#`.
- Trimmed dependency exposure by removing the unused Rust `oauth2` crate,
  moving `reqwest` to `0.12`, and disabling AVIF image features in Kujo's
  build while preserving common image formats.
- Converted the benchmark authoring checklist from unchecked task rows into
  non-blocking quality criteria, and added a Markdown hygiene contract so new
  unchecked rows outside release-flight docs are caught by tests.
- Fixed two real fixture isolation bugs in `tests/stdlib_io_test.kujo` and
  `tests/stdlib_test.kujo` so concurrent VM and dual runtime test runs no
  longer collide on shared temporary files.
- Updated README and security/stdlib docs to describe the new operator-facing
  safety behavior.
- Re-ran the full release-candidate gate locally with socket integration tests:
  `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full`.
- Refreshed `Cargo.lock` to current compatible dependency releases. This
  removed the prior `core2` and `proc-macro-error2` audit warnings while
  preserving the existing explicit `RUSTSEC-2023-0071` release-gate exception.

## Still Blocking Human v1.0 Review

1. **Release-flight artifact sign-off remains open.**
   - Source: `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
   - Blocked by: no explicit `UNBLOCK_V1_RELEASE` directive and no real
     `v1.0.0` publication event in this session.
   - Required evidence: release URLs, Linux/macOS/Windows assets, per-asset
     SHA-256 files, `checksums.txt`, published-artifact smoke workflow result,
     and dated `notes/` evidence.

## Recommended Next Session Order

1. If and only if release owners provide `UNBLOCK_V1_RELEASE`, execute
   tag-time artifact publication/sign-off.
2. Create the human review packet from current docs, command evidence, and
   release artifact state.
