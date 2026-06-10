# Kujo Field Notes — VM Compatibility and Example Stabilization

**Date:** 2026-05-05
**Session:** 10:29 local
**Branch/Commit:** main / f31fa59
**Scope:** I closed a VM/runtime compatibility gap and stabilized top-level examples so the non-benchmark suite runs reliably end-to-end. I also split and pushed the work as two meaningful commits.

---

## What I Changed
- Added VM/runtime compatibility updates in `src/vm.rs`, `src/interpreter/mod.rs`, `src/interpreter/native_functions/mod.rs`, `src/builtins.rs`, and `src/main.rs`.
- Added compatibility aliases and canonical native-name handling for legacy example usage (`println`, `str`, `dict`, `array`, `error`, `time`).
- Added VM global bootstrap for constants (`PI`, `E`, `null`) after native global registration in `src/main.rs`.
- Expanded serializer conversion handling for dict variants in `src/builtins.rs`.
- Updated many files under `examples/` to remove syntax/API drift and to make non-benchmark demos automation-safe.
- Validated core behavior with `cargo test vm_` and direct runs of `examples/test_bool.kujo` and `examples/stdlib_os.kujo`.
- Created and pushed two commits:
  - `c701d39` — `fix(vm): improve runtime compatibility and builtin bootstrap`
  - `f31fa59` — `fix(examples): make top-level demos VM-compatible`

## Gotchas (Read This Next Time)
- **Gotcha:** VM builtin bootstrap is not the same thing as VM constant bootstrap.
  - **Symptom:** Scripts/examples can fail with undefined `PI`, `E`, or `null` even when builtin globals appear initialized.
  - **Root cause:** `get_builtin_names()` drives native function global registration; constants are a separate surface and were not injected.
  - **Fix:** Explicitly set `PI`, `E`, and `null` in VM globals during run bootstrap (`src/main.rs`).
  - **Prevention:** Treat constants as a separate contract from native function names; add coverage that asserts constant availability in VM runs.

- **Gotcha:** Method behavior can still diverge between interpreter and VM if only one call path is wired.
  - **Symptom:** `obj.method(...)` behavior works in one execution mode and fails in another for non-struct values.
  - **Root cause:** Method invocation spans multiple surfaces (`Expr::MethodCall`, VM `FieldGet`, native dispatch canonicalization).
  - **Fix:** Add VM method markers and native dispatch compatibility handling in `src/vm.rs` and `src/interpreter/native_functions/mod.rs`.
  - **Prevention:** When adding/changing methods, test both interpreter and VM call paths using representative examples.

- **Gotcha:** Top-level examples are not always automation-safe by default.
  - **Symptom:** Harness runs can hang or fail on interactive/server/network demos.
  - **Root cause:** Some examples are designed for manual or long-running usage, not one-shot CI execution.
  - **Fix:** Add deterministic skip/non-blocking guards for those scenarios while preserving runnable demos.
  - **Prevention:** Keep a strict automation contract for top-level examples and isolate long-running behavior behind explicit guards.

## Things I Learned
- VM parity work is usually multi-surface: compiler/runtime/native dispatch/bootstrap all matter together.
- "Builtins registered" does not imply "runtime constants injected"; treat those as different invariants.
- Example stability is a product surface, not just test noise. Fast feedback loops come from deterministic example behavior.
- For broad compatibility updates, separating runtime commits from example commits makes review and rollback safer.

## Debug Notes (Only if applicable)
- **Failing test / error:** Non-benchmark examples previously had multiple FAIL/TIMEOUT outcomes in harness sweeps due to syntax/API drift and long-running flows.
- **Repro steps:** `timeout 12 cargo run --quiet -- run examples/*.kujo` and inspect FAIL/TIMEOUT list.
- **Breakpoints / logs used:** Iterative per-file reruns plus aggregate report parsing (`/tmp/kujo_examples_report_new6.txt`).
- **Final diagnosis:** Failures were mixed-mode: runtime dispatch/bootstrap gaps and example-level assumptions not valid for automated VM runs.

## Follow-ups / TODO (For Future Agents)
- [ ] Add a first-class CI target for non-benchmark top-level example validation.
- [ ] Decide benchmark strategy: keep separate perf-only suite or add explicit skip guards in default smoke runs.
- [ ] Debug and document current `crud-api` run failure separately from examples sweep work.

## Links / References
- Files touched:
  - `src/vm.rs`
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `src/builtins.rs`
  - `src/main.rs`
  - `examples/` (top-level non-benchmark compatibility sweep)
- Related docs:
  - `notes/FIELD_NOTES_SYSTEM.md`
  - `notes/GOTCHAS.md`
  - `README.md`
  - `ROADMAP.md`
