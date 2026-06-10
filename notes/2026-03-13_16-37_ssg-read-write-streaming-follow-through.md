# Kujo Field Notes — SSG read-to-write streaming throughput follow-through

**Date:** 2026-03-13
**Session:** 16:37 local
**Branch/Commit:** main / ee21090
**Scope:** Completed the next v0.11 P0 throughput slice by streaming `ssg_read_render_and_write_pages(...)` read completions directly into bounded render/write workers. Added regression coverage and synchronized release docs.

---

## What I Changed
- Optimized fused helper pipeline in `src/interpreter/native_functions/async_ops.rs`:
  - Removed full read-stage source-body buffering in `ssg_read_render_and_write_pages(...)`
  - Added interleaved read/write scheduling with `tokio::select!` over bounded read and write in-flight sets
  - Preserved summary output contract keys and shape: `checksum`, `files`, `read_ms`, `render_write_ms`
- Added regression coverage in `src/interpreter/native_functions/async_ops.rs`:
  - `test_ssg_read_render_and_write_pages_empty_input_returns_zero_summary`
  - `test_ssg_read_render_and_write_pages_single_worker_preserves_output_contracts`
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Executed validation commands:
  - `cargo test ssg_read_render_and_write_pages`
  - `cargo test ssg`

## Gotchas (Read This Next Time)
- **Gotcha:** Streaming read/write with `tokio::select!` can stall or mis-time stages if branch guards are wrong.
  - **Symptom:** Potential hang risk or incorrect stage metrics when one lane drains before the other.
  - **Root cause:** In a fused pipeline, read and write futures complete independently; loop/branch conditions must account for both pending reads and in-flight writes.
  - **Fix:** Use `while remaining_reads > 0 || !write_in_flight.is_empty()` with guarded select branches (`if remaining_reads > 0` and `if !write_in_flight.is_empty()`) and compute `read_ms` exactly when reads are exhausted.
  - **Prevention:** Treat fused helper control flow as a two-lane state machine (read lane + write lane) and validate with both multi-worker and single-worker contract tests.

- **Gotcha:** Existing warning in `src/vm.rs` (`unused_mut`) appears in focused SSG test runs but is unrelated to this slice.
  - **Symptom:** `cargo test ssg` prints warning at `src/vm.rs:6925` even when SSG-focused tests are green.
  - **Root cause:** Pre-existing warning in another subsystem; not introduced by SSG helper changes.
  - **Fix:** None in this slice (kept change scope focused on P0 throughput work).
  - **Prevention:** Record and defer unrelated warnings explicitly instead of mixing non-scope cleanup into throughput contract changes.

## Things I Learned
- In fused SSG helpers, throughput improvements are safer when output-contract invariants are treated as hard API boundaries (checksum/file-count + stable stage keys).
- `read_ms` and `render_write_ms` are measurement contracts for the benchmark harness, not just debug counters.
- Single-worker (`concurrency_limit=1`) regression tests are useful for catching scheduler/control-flow mistakes that may not surface under higher concurrency.

## Debug Notes (Only if applicable)
- **Failing test / error:** none in feature scope; all targeted and broader SSG tests passed.
- **Repro steps:**
  - `cargo test ssg_read_render_and_write_pages`
  - `cargo test ssg`
- **Breakpoints / logs used:** test output and contract assertions only.
- **Final diagnosis:** Streaming read-to-write scheduling preserved benchmark contracts and behavior while removing full read-stage buffering overhead.

## Follow-ups / TODO (For Future Agents)
- [ ] Capture before/after `kujo bench-ssg --profile-async --runs 5` medians for this streaming slice under stable load.
- [ ] Continue profiling residual render/write overhead inside `ssg_read_render_and_write_pages(...)` without changing benchmark key contracts.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/2026-03-13_16-37_ssg-read-write-streaming-follow-through.md`
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
