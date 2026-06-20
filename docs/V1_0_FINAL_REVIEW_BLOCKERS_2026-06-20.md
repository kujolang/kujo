# Kujo v1.0 Final Review Blockers (2026-06-20)

Status: active final-phase handoff before human v1.0 readiness review

## Current Boundary

Kujo is closer to final review after the 2026-06-20 security pass, but it is
not ready for a human v1.0 release review until the release-flight items below
are either completed with real evidence or explicitly deferred by release
owners.

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
- Updated README and security/stdlib docs to describe the new operator-facing
  safety behavior.
- Re-ran the full release-candidate gate locally with socket integration tests:
  `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full`.

## Still Blocking Human v1.0 Review

1. **Release-flight artifact sign-off remains open.**
   - Source: `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
   - Blocked by: no explicit `UNBLOCK_V1_RELEASE` directive and no real
     `v1.0.0` publication event in this session.
   - Required evidence: release URLs, Linux/macOS/Windows assets, per-asset
     SHA-256 files, `checksums.txt`, published-artifact smoke workflow result,
     and dated `notes/` evidence.

2. **Unchecked historical/future checkboxes still exist in archived notes.**
   - Source: `notes/**` and benchmark planning docs contain older future-work
     checkboxes that are not all v1 release blockers.
   - Required action: keep primary blocker tracking scoped to canonical docs
     (`ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`,
     `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`, and active v1 readiness
     checklists), or run a dedicated archive-note normalization pass to mark
     stale note checkboxes as historical.

3. **Exact human-review commit evidence must be captured after any later
   changes.**
   - This pass completed the verification bundle before commit/push, but human
     release review should capture the same commands on the exact commit being
     reviewed if any later edits land.
   - Evidence commands:
     - `cargo fmt --check`
     - `cargo check`
     - `cargo clippy --all-targets --all-features -- -D warnings`
     - `cargo test`
     - `cargo test --test docs_examples`
     - `cargo test --test readme_contracts`
     - `cargo test --test cli_contracts`
     - `cargo test --test cli_json_contracts`
     - `cargo test --test diagnostics_golden`
     - `cargo test --test native_api_security_boundaries`
     - `cargo test --test runtime_security`
     - `cargo run -- test --runtime vm`
     - `cargo run -- test --runtime dual`
     - `bash scripts/release_candidate_gate.sh --full`

## Recommended Next Session Order

1. Decide whether to normalize historical note checkboxes or keep them outside
   release-blocker accounting.
2. Re-run the verification bundle if any later changes land before human
   review.
3. If and only if release owners provide `UNBLOCK_V1_RELEASE`, execute
   tag-time artifact publication/sign-off.
4. Create the human review packet from current docs, command evidence, and
   release artifact state.
