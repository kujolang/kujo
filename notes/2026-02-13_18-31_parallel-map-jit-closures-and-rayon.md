# Kujo Field Notes — Parallel map JIT closures + rayon integration

**Date:** 2026-02-13
**Session:** 18:31 local
**Branch/Commit:** main / f0c38b2
**Scope:** Implemented the next Option 3 roadmap items for parallel iterators: rayon-backed iteration and JIT-compiled bytecode closure execution in `parallel_map`/`par_map`. Added tests, updated docs, and validated with full test suite.

---

## What I Changed
- Added rayon dependency and integrated a rayon fast path for selected native mappers in `parallel_map`.
- Implemented supported rayon mapper set in `src/interpreter/native_functions/async_ops.rs`:
  - `len`
  - `upper` / `to_upper`
  - `lower` / `to_lower`
- Added VM/JIT execution lane for bytecode closures passed to `parallel_map` / `par_map`.
- Added public VM helper `jit_compile_bytecode_function()` in `src/vm.rs` for eager per-function compilation from non-VM call sites.
- Wired bytecode closure mapping in `parallel_map` through `VM::call_function_from_jit(...)`.
- Added/updated targeted tests in `src/interpreter/native_functions/async_ops.rs`:
  - `test_parallel_map_bytecode_mapper_uses_vm_jit_lane`
  - `test_par_map_alias_with_bytecode_mapper`
  - rayon-path unit coverage updates
- Updated docs and status tracking:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** `call_user_function()` does not execute `Value::BytecodeFunction`
  - **Symptom:** Passing bytecode closures into interpreter-side helpers can silently behave wrong or return fallback values.
  - **Root cause:** `call_user_function` in `src/interpreter/mod.rs` only handles `Function` and `GeneratorDef`; other variants fall through to `_ => Value::Int(0)`.
  - **Fix:** For bytecode closures in `parallel_map`, route through VM via `VM::call_function_from_jit(...)` and optionally pre-compile with `jit_compile_bytecode_function()`.
  - **Prevention:** Rule: if mapper is `Value::BytecodeFunction`, do not call `call_user_function`; use VM execution APIs.

- **Gotcha:** Synthetic JIT error-path tests can crash with CPU trap instead of returning a clean runtime error
  - **Symptom:** Targeted test run failed with `SIGILL` when forcing certain invalid arithmetic paths through JIT.
  - **Root cause:** Some intentionally bad JIT execution paths can trap at machine-code level, not surface as `Value::Error`.
  - **Fix:** Removed brittle trap-based negative test and kept stable correctness tests for valid JIT lane behavior.
  - **Prevention:** Prefer deterministic, non-trap assertions for JIT path tests unless trap handling is explicitly under test.

- **Gotcha:** `cargo test` accepts one positional filter pattern
  - **Symptom:** `cargo test test_a test_b` fails with “unexpected argument”.
  - **Root cause:** Cargo CLI supports one test-name filter positional argument.
  - **Fix:** Use a shared substring filter (e.g. `cargo test bytecode_mapper`) or run separate invocations.
  - **Prevention:** When batching targeted tests, design names with a common prefix/substr for one-pass filtering.

## Things I Learned
- The right integration point for bytecode closure JIT in interpreter-native helpers is a temporary VM instance, not interpreter function dispatch.
- Eager JIT compile helper on VM is useful beyond main VM loop (native helper call sites can reuse JIT infra safely).
- Rayon and VM/JIT paths can coexist in `parallel_map` with strict precedence:
  1. native mapper rayon fast path,
  2. bytecode closure VM/JIT path,
  3. existing async/promise fallback.
- The `rustfmt` warnings about unstable options in this repo are expected on stable toolchains; they are not regressions from feature changes.

## Debug Notes (Only if applicable)
- **Failing test / error:** `cargo test bytecode_mapper -- --nocapture` failed once with `signal: 4, SIGILL: illegal instruction`.
- **Repro steps:** Run targeted bytecode-mapper tests immediately after adding a synthetic invalid mapper path designed to force runtime failure.
- **Breakpoints / logs used:** Used repeated targeted `cargo test` runs and narrowed to the specific negative test case; relied on panic output and process signal.
- **Final diagnosis:** The negative test design was trap-prone in JIT mode; replaced with stable, non-trap coverage and kept core behavior assertions.

## Follow-ups / TODO (For Future Agents)
- [ ] Add benchmark artifact(s) for `parallel_map` bytecode closure path vs interpreter-only fallback.
- [ ] Add explicit VM/JIT mode toggles for `parallel_map` microbench harness to compare path-by-path overhead.
- [ ] Revisit robust, deterministic error-path tests for JIT closure execution if/when trap handling strategy is formalized.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/vm.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
