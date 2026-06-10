# Kujo Field Notes — V1-PERF-001 Criterion Baseline Suite

**Date:** 2026-05-16
**Session:** 13:31 local
**Branch/Commit:** main / cd28976
**Scope:** Implemented `V1-PERF-001` by adding a Criterion-based benchmark suite for lexer, parser, interpreter, VM, module resolution, and static server request paths. Updated roadmap/docs/changelog and validated bench execution plus full test-suite health.

---

## What I Changed
- Added Criterion bench wiring in `Cargo.toml`:
  - `criterion` under `[dev-dependencies]`
  - `[[bench]] name = "v1_perf_benchmarks"` with `harness = false`
- Added new benchmark target: `benches/v1_perf_benchmarks.rs`
  - Lexer benchmarks for large source and token-heavy source.
  - Parser benchmarks for large file parsing and deep expression parsing.
  - Interpreter benchmark for loops/function calls/recursion/strings/collections.
  - VM benchmark for equivalent runtime workload.
  - Module loader benchmark for cold-loading many small modules.
  - Static server benchmarks for small and large file GET requests against live `run_static_server(...)`.
- Exposed static server module from library crate in `src/lib.rs` (`pub mod serve_http;`) so benches can target production server code paths directly.
- Updated docs/status artifacts:
  - `ROADMAP.md` marked `V1-PERF-001` complete with verification notes.
  - `README.md` added explicit `cargo bench --bench v1_perf_benchmarks` usage.
  - `CHANGELOG.md` added unreleased entry for new Criterion baselines.
- Kept repository formatting consistency after `cargo fmt` (including formatting-only updates in `tests/native_json.rs` and `tests/stdlib_reference_contract.rs`).

## Gotchas (Read This Next Time)
- **Gotcha:** VM benchmark workloads need builtin globals explicitly seeded in custom bench harnesses.
  - **Symptom:** VM benchmark panicked with `Undefined variable: len`.
  - **Root cause:** Fresh `VM::new()` in bench code does not automatically have native function globals seeded the same way interpreter paths do for this workload.
  - **Fix:** Added a `configure_vm_globals(...)` helper in `benches/v1_perf_benchmarks.rs` that registers `Interpreter::get_builtin_names()` as `Value::NativeFunction(...)` before `vm.execute(...)`.
  - **Prevention:** Reuse benchmark runner global-seeding pattern whenever VM benches execute code that can lower to builtins (`len`, `range`, etc.).

- **Gotcha:** `for i in <int>` and `for i in range(<int>)` are not interchangeable for all VM benchmark code paths.
  - **Symptom:** VM benchmark panicked with `len() requires an array, dict, bytes, set, queue, stack, or string`.
  - **Root cause:** Benchmark loop form `for i in 32` hit an iteration contract path that expected a collection-like input.
  - **Fix:** Switched benchmark loops to `for i in range(32)` / `for i in range(40)` to keep interpreter/VM workload semantics aligned.
  - **Prevention:** Prefer explicit `range(...)` in synthetic loop-heavy benchmark sources intended for parity across runtimes.

- **Gotcha:** Busy local machine load can make Criterion trend messages noisy or misleading.
  - **Symptom:** Large timing deltas and outlier warnings across repeated short-run measurements.
  - **Root cause:** Local resource contention (CPU/memory/process churn) perturbs microbenchmark timing more than code-level changes.
  - **Fix:** Treated benchmark execution in this session as smoke/shape validation evidence, not a release-grade performance claim.
  - **Prevention:** For performance comparisons, run on a controlled/idle host with longer measurement windows and stable environment.

## Things I Learned
- The existing Kujo repo had benchmarking features (`kujo bench`, `bench-ssg`) but no Criterion-driven `cargo bench` suite for core runtime surfaces; this item fills that gap cleanly.
- Static-server benchmarking can be kept deterministic enough for local smoke by creating one long-lived fixture server and measuring request paths only.
- For roadmap wording, explicit verification commands in `Notes:` are useful to clarify that performance numbers are environment-sensitive and that successful execution is the key contract for this item.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm benchmark workload should execute: "Undefined variable: len"`
- **Repro steps:** `cargo bench --bench v1_perf_benchmarks -- --warm-up-time 0.01 --measurement-time 0.01 --sample-size 10`
- **Breakpoints / logs used:** Criterion panic location (`benches/v1_perf_benchmarks.rs`) and existing benchmark-runner VM global-registration pattern in `src/benchmarks/runner.rs`.
- **Final diagnosis:** Bench harness needed explicit VM builtin registration plus loop-shape normalization (`range(...)`) for runtime-parity benchmark source.

## Follow-ups / TODO (For Future Agents)
- [ ] Use this baseline suite to drive `V1-PERF-002` hotspot audits with controlled host measurements.
- [ ] Evaluate adding CI/nightly benchmark smoke invocation with tuned Criterion flags once runtime budget is acceptable.

## Links / References
- Files touched:
  - `Cargo.toml`
  - `src/lib.rs`
  - `benches/v1_perf_benchmarks.rs`
  - `ROADMAP.md`
  - `README.md`
  - `CHANGELOG.md`
  - `tests/native_json.rs`
  - `tests/stdlib_reference_contract.rs`
- Related docs:
  - `ROADMAP.md`
  - `README.md`
  - `CHANGELOG.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
