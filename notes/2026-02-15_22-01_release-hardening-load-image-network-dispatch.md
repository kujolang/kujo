# Kujo Field Notes — Release hardening for image and network dispatch

**Date:** 2026-02-15
**Session:** 22:01 local
**Branch/Commit:** main / 5e064df
**Scope:** Closed remaining v0.10 modular native dispatch gaps for `load_image` and all declared TCP/UDP APIs. Added dispatcher + behavior contract coverage, updated roadmap/changelog/readme, and pushed incremental commits.

---

## What I Changed
- Added modular image dispatch handler in `src/interpreter/native_functions/filesystem.rs`:
  - `load_image(path)` with format detection from extension and legacy-compatible error message shape.
- Implemented modular TCP/UDP dispatch in `src/interpreter/native_functions/network.rs`:
  - `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_send`, `tcp_receive`, `tcp_close`, `tcp_set_nonblocking`
  - `udp_bind`, `udp_send_to`, `udp_receive_from`, `udp_close`
- Expanded release-hardening dispatcher contracts in `src/interpreter/native_functions/mod.rs`:
  - Added `load_image` and network APIs to recent critical coverage.
  - Removed migrated entries from known legacy dispatch gaps.
  - Added argument-shape/error-shape contract tests.
  - Added successful runtime behavior tests:
    - `load_image` round-trip (format + dimensions)
    - TCP end-to-end send/receive and nonblocking toggle contracts
    - UDP end-to-end send/receive dictionary-shape contract
- Updated milestone docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Validation run:
  - targeted `cargo test` for new contracts and drift guards
  - full `cargo build` and `cargo test`

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo fmt` repeatedly reformats unrelated native-function modules, even when only one module is changed.
  - **Symptom:** `git status --short` showed modifications in `async_ops.rs`, `crypto.rs`, `json.rs`, and `system.rs` after formatting network/image-only work.
  - **Root cause:** workspace rustfmt settings plus module-wide formatting normalization can touch adjacent files that were not part of the feature slice.
  - **Fix:** Immediately run `git restore` on unrelated files before staging commits.
  - **Prevention:** Treat `cargo fmt` as required validation, but explicitly re-scope the working tree to feature files before each commit.

- **Gotcha:** Dispatcher drift guard is a synchronized ledger, not a static test.
  - **Symptom:** After migration, the declared-builtin drift test fails if `expected_known_legacy_dispatch_gaps` is not updated.
  - **Root cause:** Contract test intentionally encodes current known gaps; migration work must remove newly covered APIs from that list.
  - **Fix:** Remove migrated API names from `expected_known_legacy_dispatch_gaps` in `src/interpreter/native_functions/mod.rs` and add recent-API coverage entries.
  - **Prevention:** Any modular dispatch migration must update both: (1) recent critical coverage list, and (2) known-gap list.

## Things I Learned
- Network coverage should use ephemeral ports (`127.0.0.1:0`) to avoid flaky bind conflicts in CI and local runs.
- For TCP contract tests, concurrent client/server logic can stay deterministic by creating the listener first, then joining the client thread after server-side assertions.
- `udp_receive_from` contract should verify dict shape (`data`, `from`, `size`) rather than only payload content.
- Release-hardening changes are safest when split into three commits: implementation, contract tests, docs.

## Debug Notes (Only if applicable)
- **Failing test / error:** None in final state; no persistent failures after migration.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** N/A.
- **Final diagnosis:** Main friction was commit scoping after formatter spillover, not runtime correctness.

## Follow-ups / TODO (For Future Agents)
- [ ] If additional network behavior is added (timeouts, DNS helpers, socket options), extend both recent-API and drift-ledger contracts in `src/interpreter/native_functions/mod.rs`.
- [ ] Keep native network tests constrained to localhost and ephemeral ports to maintain deterministic CI behavior.
- [ ] Re-check `notes/GOTCHAS.md` periodically to keep dispatch-ledger guidance deduplicated.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/filesystem.rs`
  - `src/interpreter/native_functions/network.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
