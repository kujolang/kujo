# Kujo Field Notes — V1-PERF-002 module-loader cycle-check scaling

**Date:** 2026-05-16
**Session:** 13:48 local
**Branch/Commit:** main / pending
**Scope:** Completed `V1-PERF-002` by removing avoidable O(n^2) behavior in module-load cycle detection and adding regression/perf coverage for deep import-chain workloads.

---

## What I Changed
- Replaced linear circular-import detection scans in `src/module.rs` with an O(1) `loading_stack_index: HashMap<ModuleCacheKey, usize>` lookup.
- Updated module loader push/pop flow to keep `loading_stack` and `loading_stack_index` synchronized.
- Added module-loader regressions in `src/module.rs` tests:
  - `load_module_detects_circular_imports` now also asserts loading-stack/index cleanup after errors.
  - `load_module_deep_chain_completes_and_clears_loading_index` validates successful chained imports and cleanup after success.
- Added a dedicated Criterion hotspot benchmark in `benches/v1_perf_benchmarks.rs`:
  - `module_resolution/deep_import_chain_cold_loader`.
- Updated roadmap/changelog entries for `V1-PERF-002` completion.

## Gotchas (Read This Next Time)
- **Gotcha:** Deep module import chains can overflow stack in debug-style test runs before explicit recursion/call-depth safeguards are in place.
  - **Symptom:** `thread '...load_module_deep_chain...' has overflowed its stack` when stress depth was too high.
  - **Root cause:** Module loading/evaluation remains recursively nested across import chains, so very deep synthetic chains trip host stack limits.
  - **Fix:** Kept regression/benchmark chain sizes moderate for this item (`12` in test, `24` in bench fixture) while still exercising cycle-check scaling.
  - **Prevention:** When writing import-depth stress tests before `V1-PERF-003`, use bounded depths and avoid conflating recursion-limit work with cycle-detection complexity work.

## Things I Learned
- The previous cycle check used `loading_stack.iter().position(...)` on every load, which creates avoidable O(n^2) behavior for acyclic chain imports.
- A side index map is enough to eliminate that quadratic pattern without changing import semantics or diagnostics.
- Stack/index cleanup assertions are important because failed imports can exit through multiple nested paths.

## Debug Notes (Only if applicable)
- **Failing test / error:** `thread 'module::tests::load_module_deep_chain_completes_and_clears_loading_index' has overflowed its stack`
- **Repro steps:** `cargo test --lib load_module_` with higher deep-chain fixture counts.
- **Breakpoints / logs used:** Iterative depth reduction and reruns; no runtime instrumentation needed.
- **Final diagnosis:** Recursion depth, not semantic correctness, caused failure. Bounded fixture depth retained target coverage.

## Follow-ups / TODO (For Future Agents)
- [ ] Close `V1-PERF-003` recursion/call-depth safeguards so deeper import-chain stress can be tested safely.
- [ ] Consider adding a larger deep-chain benchmark variant behind an opt-in flag once recursion limits are explicit and configurable.

## Links / References
- Files touched:
  - `src/module.rs`
  - `benches/v1_perf_benchmarks.rs`
  - `ROADMAP.md`
  - `CHANGELOG.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
