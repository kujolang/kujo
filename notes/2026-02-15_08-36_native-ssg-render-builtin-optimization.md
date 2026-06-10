# Kujo Field Notes — Native bulk SSG render optimization

**Date:** 2026-02-15
**Session:** 08:36 local
**Branch/Commit:** main / a136822
**Scope:** Implemented the next roadmap optimization for SSG throughput by offloading high-volume HTML page wrapping to a native builtin. Added coverage and updated benchmark/docs artifacts to reflect measured impact.

---

## What I Changed
- Added `ssg_render_pages(source_pages)` native builtin in `src/interpreter/native_functions/strings.rs`.
- Registered the new builtin in `src/interpreter/mod.rs` (`available_native_functions()` + `register_builtins()`).
- Updated `benchmarks/cross-language/bench_ssg.kujo` to replace per-item HTML wrapping loop with native bulk rendering:
  - `rendered := ssg_render_pages(source_pages)`
  - `html_pages := rendered["pages"]`
  - `checksum := rendered["checksum"]`
- Added comprehensive unit tests in `src/interpreter/native_functions/strings.rs` for:
  - valid render + checksum behavior
  - non-array input validation
  - non-string element validation
  - argument count validation
- Updated docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `bench-ssg` render/write stage cost is dominated by Kujo-side string construction, not read throughput.
  - **Symptom:** `--profile-async` showed read stage in sub-second range while render/write consumed >98% of profiled time.
  - **Root cause:** Building 10,000 HTML strings in Kujo script loops causes substantial interpreter overhead before async file writes.
  - **Fix:** Move page wrapping into native Rust (`ssg_render_pages`) and keep async writes bulked through `async_write_files(...)`.
  - **Prevention:** For high-volume hot loops doing deterministic string shaping, consider a native bulk path before tuning async scheduling knobs.

- **Gotcha:** Cross-language benchmark comparisons are noisy enough to hide Kujo-side wins.
  - **Symptom:** `bench-ssg --compare-python` varied significantly between consecutive local runs.
  - **Root cause:** Combined cross-language workload introduces filesystem cache and contention variability.
  - **Fix:** Use Kujo-only `bench-ssg --profile-async` before/after for optimization validation, then use compare-python as a trend snapshot.
  - **Prevention:** Treat one-off compare-python values as directional; avoid using a single run as optimization proof.

## Things I Learned
- Rule: In this SSG benchmark, page rendering overhead is the primary bottleneck; optimizing Promise orchestration alone has diminishing returns.
- `ssg_render_pages(...)` is a practical bridge step while larger VM/runtime architecture work continues.
- Profile stage-level breakdown first; choose optimization targets from measured stage dominance, not intuition.

## Debug Notes (Only if applicable)
- **Failing test / error:** None.
- **Repro steps:** N/A.
- **Breakpoints / logs used:** `cargo run --quiet -- bench-ssg --profile-async` and `cargo run --quiet -- bench-ssg --compare-python`.
- **Final diagnosis:** Render/write stage remains dominant; native bulk rendering reduced this path materially but does not yet meet roadmap latency targets.

## Follow-ups / TODO (For Future Agents)
- [ ] Add an indexed-path bulk helper (native) to remove remaining benchmark-script path-building loops.
- [ ] Evaluate moving output-path generation + write scheduling into a single native bulk API to reduce script-level orchestration overhead further.
- [ ] Collect 5-run median benchmark sampling in `bench-ssg` command output to reduce noise when comparing optimization steps.

## Links / References
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/strings.rs`
  - `benchmarks/cross-language/bench_ssg.kujo`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
