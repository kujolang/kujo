# Kujo Field Notes — SSG single-worker fast path

**Date:** 2026-03-17
**Session:** 12:19 local
**Branch/Commit:** main / 97b1a4d
**Scope:** Captured the earlier v0.11 P0 throughput work that introduced dedicated single-worker execution lanes for SSG write pipelines and validated contract compatibility.

---

## What I Changed
- Added dedicated single-worker (`concurrency_limit=1`) execution lanes in `src/interpreter/native_functions/async_ops.rs` for:
  - `ssg_render_and_write_pages(...)`
  - `ssg_read_render_and_write_pages(...)`
- Preserved checksum/file-count contracts and stage metric keys.
- Added focused regression coverage for single-worker render/write contract preservation in `src/interpreter/native_functions/async_ops.rs`.
- Updated release docs for the completed slice in:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Single-worker fast paths can accidentally change stage timing semantics
  - **Symptom:** It is easy to return `render_write_ms = 0` for non-empty single-worker runs when bypassing the multi-lane state machine.
  - **Root cause:** Fast-path branches skip the shared timing lifecycle used by the multi-worker path unless timing is explicitly reintroduced.
  - **Fix:** Start render/write timing at first successful read and compute elapsed time at completion, even in single-worker mode.
  - **Prevention:** Treat `read_ms` and `render_write_ms` keys as compatibility contracts for both multi-worker and single-worker paths.

- **Gotcha:** Fast-path optimizations must preserve error text and failure propagation shape
  - **Symptom:** Small branch-specific error rewrites can break existing contract tests and user-facing diagnostics.
  - **Root cause:** New fast-path code can diverge from established error format patterns (`Failed to read file ...`, `Failed to write file ...`).
  - **Fix:** Reused existing error text format and index/path context in fast-path branches.
  - **Prevention:** Mirror legacy error shape verbatim unless intentionally changing public contract with synchronized tests/docs.

## Things I Learned
- A single-worker lane is still a contract-bearing execution mode, not just a performance shortcut.
- Throughput optimizations in this area are safest when they keep output, checksum, stage keys, and error text byte-compatible.
- Focused tests (`ssg_render_and_write_pages`, `ssg_read_render_and_write_pages`) catch most contract drift quickly.

## Debug Notes (Only if applicable)
- **Failing test / error:** No failing tests in this slice; risk was semantic drift in `render_write_ms` under single-worker execution.
- **Repro steps:** Run single-worker paths with `cargo test ssg_render_and_write_pages` and `cargo test ssg_read_render_and_write_pages` and inspect summary fields.
- **Breakpoints / logs used:** Promise resolution payload inspection through existing test assertions.
- **Final diagnosis:** Timing and error-shape compatibility needed explicit handling in single-worker branches.

## Follow-ups / TODO (For Future Agents)
- [ ] Keep adding paired tests for any future branch-specific fast paths (`concurrency_limit=1` vs `>1`).
- [ ] If stage metrics are ever refactored, assert parity contracts across both lanes before merge.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
