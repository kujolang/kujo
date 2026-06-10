# Kujo Field Notes — SSG checksum write-result accounting

**Date:** 2026-03-16
**Session:** 16:23 local
**Branch/Commit:** main / 5629cce, c13bfed
**Scope:** Completed a throughput follow-through slice for SSG helpers by removing pre-write checksum precomputation and using actual async write-result bytes for checksum accounting, then added Unicode-focused regression coverage and updated release docs.

---

## What I Changed
- Updated `src/interpreter/native_functions/async_ops.rs`:
  - `ssg_render_and_write_pages(...)` now accumulates checksum from `ssg_write_rendered_html_page(...)` write results instead of precomputing from scheduled body lengths.
  - `ssg_read_render_and_write_pages(...)` removed precomputed rendered-length queueing (`pending_writes` no longer carries `html_len`) and now accumulates checksum from write-result lengths.
- Added checksum regressions in `src/interpreter/native_functions/async_ops.rs`:
  - `test_ssg_render_and_write_pages_unicode_checksum_matches_written_outputs`
  - `test_ssg_read_render_and_write_pages_unicode_checksum_matches_written_outputs`
- Updated docs for the milestone:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** If checksum is computed from scheduled string-length assumptions, drift risk grows when rendering/write pathways evolve.
  - **Symptom:** Throughput refactors can preserve file contents but still make checksum logic brittle to scheduling or representation changes.
  - **Root cause:** Pre-write accounting couples checksum math to pre-dispatch assumptions instead of final write outcomes.
  - **Fix:** Use returned bytes written from `ssg_write_rendered_html_page(...)` as the checksum source of truth.
  - **Prevention:** Keep checksum accumulation in write-completion paths for both SSG helpers.

- **Gotcha:** Refactoring tuple payloads in fused queue/worker paths is easy to miss in one branch.
  - **Symptom:** Compile errors like tuple-arity mismatch in `pending_writes.pop_front()` branches.
  - **Root cause:** `pending_writes` shape changed from `(usize, String, usize)` to `(usize, String)` but one refill branch still destructured the old tuple.
  - **Fix:** Update all enqueue/dequeue destructuring and `make_write_future(...)` callsites together.
  - **Prevention:** When changing queue item shape, search all `pop_front()` / `push_back()` / worker closures in the same function before rebuilding.

## Things I Learned
- Rule: for output-equivalence contracts, byte-accurate checksum from actual write completion is more robust than pre-write estimations.
- Unicode/multibyte coverage is a high-value checksum contract guard for SSG benchmark paths.
- For fused async pipelines, queue tuple shapes are an implicit contract across read, write, and refill branches.

## Debug Notes (Only if applicable)
- **Failing test / error:** Build initially failed with tuple mismatch after queue payload refactor.
- **Repro steps:**
  - `cargo fmt`
  - `cargo build`
  - `cargo test ssg_render_and_write_pages_unicode_checksum_matches_written_outputs`
  - `cargo test ssg_read_render_and_write_pages_unicode_checksum_matches_written_outputs`
- **Breakpoints / logs used:** Compiler diagnostics only (type mismatch in destructuring/call arity).
- **Final diagnosis:** One `pending_writes` refill branch still used stale `(index, body, html_len)` tuple destructuring.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a focused helper-level contract test asserting write-result accounting remains the checksum source if `ssg_write_rendered_html_page(...)` internals change.
- [ ] Consider unifying repeated queue-drain snippets in fused path to reduce tuple-shape drift risk.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
