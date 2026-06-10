# Kujo Field Notes — V1-HTTP-006 Streaming Static Serve Responses

**Date:** 2026-05-15
**Session:** 13:58 local
**Branch/Commit:** main / pending
**Scope:** Implemented roadmap item V1-HTTP-006 by converting static file responses from in-memory buffering to streamed readers, preserving range behavior and hardening serve integration test stability.

---

## What I Changed
- Reworked `src/serve_http.rs` response planning to store body readers (`Box<dyn Read + Send>`) instead of `Vec<u8>` payloads.
- Added `open_file_nofollow(...)` to open and validate regular files while returning file length metadata for response headers and range validation.
- Changed static serve happy-path file responses to stream from file handles, including ranged responses via `seek + take`.
- Preserved deterministic `Content-Length` behavior for streamed responses by forcing identity transfer behavior (`with_chunked_threshold(usize::MAX)`).
- Added `tests/serve_command_integration.rs` regressions for:
  - invalid range (`416` + `Content-Range`),
  - large-file GET length/body contract,
  - large-file HEAD no-body + `Content-Length` contract.
- Fixed a readiness race in `spawn_serve_process_with_extra_args(...)` where a parallel test process could make a different server appear "ready" for the wrong child process.
- Updated `README.md`, `CHANGELOG.md`, and `ROADMAP.md` for V1-HTTP-006 completion.

## Gotchas (Read This Next Time)
- **Gotcha:** `tiny_http` can switch large responses to chunked transfer, which breaks deterministic raw-socket body parsing expectations in integration tests.
  - **Symptom:** Large-file GET body length exceeded expected payload and `Content-Length` header became missing/inconsistent.
  - **Root cause:** Response writer selected chunked behavior for large payloads.
  - **Fix:** Set `.with_chunked_threshold(usize::MAX)` when constructing serve responses.
  - **Prevention:** When tests parse raw HTTP bodies directly (without chunk decoding), pin transfer behavior and assert `Content-Length` explicitly.
- **Gotcha:** Parallel serve integration tests can false-positive readiness against another test process if port reuse races occur.
  - **Symptom:** Random `Connection refused` in unrelated serve tests during full `cargo test` runs.
  - **Root cause:** Readiness check returned on successful TCP connect before verifying the spawned child was still alive.
  - **Fix:** Check `child.try_wait()` before and after readiness connect success.
  - **Prevention:** In subprocess-based tests, validate process liveness at every readiness checkpoint.

## Things I Learned
- In Kujo’s serve path, range/header/body correctness can be preserved while switching to streaming if response length metadata is carried independently from body materialization.
- The `serve_command_integration` harness is sensitive to cross-test process races because it uses real subprocesses and real sockets; small readiness-order changes significantly affect flakiness.
- Keeping range support while streaming avoids an unnecessary semantics rollback and keeps docs/contracts stable.

## Debug Notes (Only if applicable)
- **Failing test / error:** `failed to read HTTP response after retries: failed to connect to serve process: Connection refused (os error 61)` during `cargo test` in `serve_command_integration`.
- **Repro steps:** Run `cargo test` repeatedly with parallel test execution; failures appeared in different serve tests.
- **Breakpoints / logs used:** Checked serve test harness readiness logic and compared child-process liveness checks around `TcpStream::connect(...)`.
- **Final diagnosis:** Port race/readiness false positive in test harness; not a deterministic runtime logic failure in request handling.

## Follow-ups / TODO (For Future Agents)
- [ ] If serve integration flakes reappear, consider making the test binary single-threaded by default or adding a shared test-level server spawn lock.
- [ ] If future requirements expand Range support (multi-range/suffix edge behavior), extend `parse_single_range(...)` contracts and add explicit compliance tests.

## Links / References
- Files touched:
  - `src/serve_http.rs`
  - `tests/serve_command_integration.rs`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
