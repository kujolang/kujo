# Kujo Field Notes — bench-cross working-directory gotcha

**Date:** 2026-02-13
**Session:** 18:52 local
**Branch/Commit:** main / f2d5437
**Scope:** Validated how to run the new `bench-cross` CLI workflow and investigated the user-reported failure path. Documented the non-obvious CWD constraint and safe invocation patterns.

---

## What I Changed
- Reproduced failure from `benchmarks/cross-language`:
  - `cargo run --release -- bench-cross`
- Captured exact CLI error output:
  - `Kujo benchmark script not found: benchmarks/cross-language/bench_parallel_map.kujo`
- Confirmed root-cause is path resolution relative to process CWD, not binary location.
- Added a curated gotcha entry in `notes/GOTCHAS.md` under CLI/tooling behavior.
- Added this session to `notes/README.md` index.

## Gotchas (Read This Next Time)
- **Gotcha:** `bench-cross` default script paths are relative to current working directory.
  - **Symptom:** Running from `benchmarks/cross-language` fails with `Kujo benchmark script not found: benchmarks/cross-language/bench_parallel_map.kujo`.
  - **Root cause:** Defaults are configured as relative paths in clap (`benchmarks/cross-language/...`) and are evaluated from runtime CWD.
  - **Fix:** Run from repo root (`/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo`) or pass explicit absolute/relative overrides:
    - `cargo run --release -- bench-cross --kujo-script ./bench_parallel_map.kujo --python-script ./bench_process_pool.py`
  - **Prevention:** Treat `bench-cross` defaults as repo-root-relative until command-side path normalization is implemented.

## Things I Learned
- Rule: For Kujo CLI commands that embed relative default paths, command behavior can change by shell location even when binary path is fixed.
- The failure mode is expected and safe; it is a path contract issue, not a runtime/interpreter regression.
- Always include exact failing command + exact stderr in notes when documenting CLI gotchas.

## Debug Notes (Only if applicable)
- **Failing test / error:**
  - `Kujo benchmark script not found: benchmarks/cross-language/bench_parallel_map.kujo`
- **Repro steps:**
  - `cd /Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/benchmarks/cross-language`
  - `cargo run --release -- bench-cross`
- **Breakpoints / logs used:**
  - CLI stderr output only (no code-level debugging required).
- **Final diagnosis:**
  - CWD-relative default benchmark paths caused lookup failure outside repo root.

## Follow-ups / TODO (For Future Agents)
- [ ] Normalize `bench-cross` default script paths against project root or executable root in CLI code.
- [ ] Add a small CLI regression test for bench-cross path resolution behavior.
- [ ] Consider emitting actionable hint on not-found errors (e.g., “run from repo root or pass --kujo-script/--python-script”).

## Links / References
- Files touched:
  - `notes/2026-02-13_18-52_bench-cross-cwd-gotcha.md`
  - `notes/README.md`
  - `notes/GOTCHAS.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `src/main.rs`