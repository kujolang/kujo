# Kujo Field Notes — SSG precomputed-prefix throughput + full-suite test stability

**Date:** 2026-03-15
**Session:** 16:38 local
**Branch/Commit:** main / 1ff9359
**Scope:** Continued v0.11 P0 SSG throughput follow-through by precomputing per-index HTML prefixes for fused streamed write workers, then stabilized a full-suite flaky direct writer contract test.

---

## What I Changed
- Added precomputed per-index render-prefix helper and constants in `src/interpreter/native_functions/async_ops.rs`:
  - `SSG_HTML_PREFIX_START`, `SSG_HTML_PREFIX_END`, `SSG_HTML_SUFFIX`
  - `ssg_build_render_prefixes_for_batch(file_count)`
- Updated `ssg_write_rendered_html_page(...)` to accept a precomputed prefix string instead of formatting index text per write.
- Updated fused write-worker paths (`ssg_render_and_write_pages(...)` and `ssg_read_render_and_write_pages(...)`) to reuse precomputed prefixes by index.
- Added/updated tests in `src/interpreter/native_functions/async_ops.rs`:
  - `test_ssg_build_render_prefixes_for_batch_generates_stable_prefixes`
  - `test_ssg_render_and_write_pages_preserves_large_index_heading_contract`
  - Updated direct helper tests for new signature.
- Stabilized `test_ssg_write_rendered_html_page_streams_exact_content_and_length` with a short bounded retry-on-length check before asserting final full content.
- Updated release docs for this milestone:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Direct streamed-writer helper test can fail only under full-suite load, even when isolated runs are green.
  - **Symptom:** `test_ssg_write_rendered_html_page_streams_exact_content_and_length` intermittently read content missing the closing suffix `</article></body></html>` during full `cargo test`.
  - **Root cause:** Immediate post-write file read in test proved timing-sensitive under broad concurrent test load (not reproduced consistently in single-test runs).
  - **Fix:** In the test only, added a short bounded retry loop that waits until observed file length matches the helper-reported `html_len` before asserting full string equality.
  - **Prevention:** Keep direct async file-writer tests resilient to load-sensitive visibility timing; preserve strict full-content assertions, but gate assertion on expected-length readiness first.

- **Gotcha:** Throughput optimizations in fused SSG helpers can accidentally drift output contracts if string assembly logic is duplicated.
  - **Symptom:** Prefix/suffix/index formatting changes are easy to spread across multiple write lanes and helpers.
  - **Root cause:** Fused paths (`ssg_render_and_write_pages` + `ssg_read_render_and_write_pages`) each manage async write workers and can diverge if header/footer formatting is assembled ad hoc.
  - **Fix:** Centralized constants + batch prefix precompute helper and reused them across both helpers.
  - **Prevention:** Treat HTML framing pieces (`prefix/index/article open` + suffix) as shared contract primitives; don’t hand-roll per-path formatting.

## Things I Learned
- For this codebase, “optimization complete” for SSG paths means both:
  - throughput-side reduction (allocation/format churn), and
  - unchanged checksum/file-count + stage-metric contracts.
- The direct helper test layer is valuable because it catches subtle writer-behavior drift that higher-level checksum tests can miss.
- Adding deterministic uniqueness to temp paths is useful, but it does not eliminate all full-suite timing sensitivity around immediate file reads.

## Debug Notes (Only if applicable)
- **Failing test / error:** `interpreter::native_functions::async_ops::tests::test_ssg_write_rendered_html_page_streams_exact_content_and_length`
- **Repro steps:**
  1. Run full suite: `cargo test`
  2. Observe intermittent failure in direct helper test
  3. Re-run isolated: `cargo test test_ssg_write_rendered_html_page_streams_exact_content_and_length -- --nocapture` (often passes)
- **Breakpoints / logs used:** Used assertion diff output + targeted reruns; no debugger breakpoints needed.
- **Final diagnosis:** Full-suite-only timing sensitivity in immediate read-after-write test assertion, addressed with bounded readiness polling in test.

## Follow-ups / TODO (For Future Agents)
- [ ] Re-run `kujo bench-ssg --profile-async --runs 3` and compare stage medians before/after precomputed-prefix follow-through.
- [ ] Keep evaluating remaining residual render/write overhead opportunities listed in `ROADMAP.md` without changing benchmark metric-key contracts.

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
