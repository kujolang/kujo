# Kujo Field Notes — bench-ssg trend drift warning signals

**Date:** 2026-03-16
**Session:** 16:21 local
**Branch/Commit:** main / b90c01c
**Scope:** Added trend-drift warning signals for `bench-ssg` measured-run trend output and wired warning presentation through CLI reporting. Added focused regression coverage and synchronized release docs.

---

## What I Changed
- Added `SSG_TREND_WARNING_THRESHOLD_PERCENT` and trend warning collectors in `src/benchmarks/ssg.rs`:
  - `collect_trend_warning(...)`
  - `collect_ssg_trend_warnings(...)`
- Added trend-warning contract tests in `src/benchmarks/ssg.rs` for:
  - warning emission on large deltas
  - suppression for low run counts
  - suppression for sub-threshold or percentless deltas
- Updated `bench-ssg` CLI flow in `src/main.rs` to print a `Trend stability warnings:` section when trend drift warnings exist.
- Updated release docs in `CHANGELOG.md`, `ROADMAP.md`, and `README.md` for the milestone.

## Gotchas (Read This Next Time)
- **Gotcha:** Trend reporting and trend warning gating are intentionally different.
  - **Symptom:** Trend lines can print for `--runs 2`, but no trend stability warnings appear.
  - **Root cause:** Trend generation requires `>= 2` measured runs, while trend warning emission is intentionally gated to `>= 3` measured runs to avoid noisy first/last over-interpretation.
  - **Fix:** Keep the split contract: show directional trend early, warn only with stronger sample count.
  - **Prevention:** Do not unify these thresholds without updating tests/docs and revalidating benchmark interpretation policy.

- **Gotcha:** `cargo fmt` can introduce unrelated one-line reflows outside the current feature scope.
  - **Symptom:** `git status` showed a change in `src/interpreter/native_functions/async_ops.rs` after benchmark-only edits.
  - **Root cause:** Formatter normalized formatting in an unrelated file touched by workspace-wide formatting.
  - **Fix:** Restore unrelated files (`git checkout -- <file>`) before staging feature commits.
  - **Prevention:** Always inspect and re-scope the tree immediately after `cargo fmt`.

## Things I Learned
- `bench-ssg` measurement-quality signals are now split into:
  - distribution stability (`collect_ssg_variability_warnings(...)` on aggregates)
  - directional drift stability (`collect_ssg_trend_warnings(...)` on first→last trend report)
- For benchmark UX, warning text is much easier to maintain when warning generation stays in `src/benchmarks/ssg.rs` and `src/main.rs` only handles rendering.
- Rule: warning contracts should be explicit in tests for both positive and negative paths (emit + suppress), not just one side.

## Debug Notes (Only if applicable)
- **Failing test / error:** No failing tests after implementation; targeted benchmark tests passed.
- **Repro steps:**
  - `cargo fmt`
  - `cargo build`
  - `cargo test collect_ssg_trend_warnings`
  - `cargo test analyze_ssg_benchmark_trends`
  - `cargo test collect_ssg_variability_warnings`
- **Breakpoints / logs used:** N/A (unit-contract changes validated with focused tests).
- **Final diagnosis:** Implementation was correct; only cleanup needed was reverting unrelated formatter drift in `src/interpreter/native_functions/async_ops.rs`.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a small end-to-end CLI output snapshot test for combined trend + warning sections if command-output contract testing is expanded.
- [ ] Revisit trend warning threshold tuning (`10%`) after collecting more stable multi-run benchmark histories.

## Links / References
- Files touched:
  - `src/benchmarks/ssg.rs`
  - `src/main.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
