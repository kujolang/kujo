# Kujo Field Notes — SSG Vectored Write Byte Accounting

**Date:** 2026-03-16
**Session:** 16:58 local
**Branch/Commit:** main / 5283eae
**Scope:** Implemented the next v0.11 P0 throughput slice by tightening streamed SSG write accounting to use observed vectored write-byte totals directly. Added focused regression coverage and synchronized CHANGELOG/ROADMAP/README.

---

## What I Changed
- Updated `src/interpreter/native_functions/async_ops.rs`:
  - Removed post-write rendered-length recomputation from `ssg_write_rendered_html_page(...)` return path.
  - Accumulated and returned `total_written` directly from `write_vectored(...)` loop results.
  - Replaced old rendered-length helper tests with direct writer byte-accounting regression tests.
- Added focused tests for streamed writer byte accounting:
  - `test_ssg_write_rendered_html_page_returns_total_written_bytes`
  - `test_ssg_write_rendered_html_page_returns_utf8_written_bytes`
- Updated release documentation:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `cargo test` accepts only one positional test filter.
  - **Symptom:** Running `cargo test <filter1> <filter2>` failed with `unexpected argument`.
  - **Root cause:** Cargo CLI expects a single positional test-name filter; additional names are parsed as invalid arguments.
  - **Fix:** Used one shared filter (`cargo test ssg_`) and focused single-filter runs for targeted checks.
  - **Prevention:** When validating multiple tests, use one broader filter or run separate commands per filter.

- **Gotcha:** “Write-result accounting” can still drift conceptually if return values are recomputed from expected lengths.
  - **Symptom:** Helper contract said “write-result accounting,” but return value was still derived from precomputed lengths.
  - **Root cause:** `ssg_write_rendered_html_page(...)` previously returned deterministic length math after write completion.
  - **Fix:** Return accumulated byte totals from actual `write_vectored(...)` results.
  - **Prevention:** For throughput/correctness follow-through, prefer observed I/O result values over recomputed equivalents when contracts allow.

## Things I Learned
- In this SSG path, observed-byte accounting in the write helper is the narrowest place to enforce “actual write result” semantics without changing public stage-metric keys.
- Existing SSG contract coverage is broad enough that focused writer tests plus the `ssg_` suite provide strong confidence for this optimization slice.
- UTF-8 byte-count tests are valuable for this helper because Rust string `.len()` is byte length, and this path is explicitly byte-accounted.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error: unexpected argument 'test_ssg_write_rendered_html_page_returns_utf8_written_bytes' found`
- **Repro steps:** Run `cargo test <name1> <name2> -- --nocapture`.
- **Breakpoints / logs used:** CLI output inspection only.
- **Final diagnosis:** Cargo test command shape issue (single positional filter), not a code regression.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue profiling residual `bench-ssg` render/write overhead after direct write-byte return tightening.
- [ ] Consider whether explicit `flush()` in streamed write helper is still needed for benchmark-only paths without changing correctness guarantees.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
