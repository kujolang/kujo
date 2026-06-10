# Kujo Field Notes — Release hardening safe probe for input/exit

**Date:** 2026-02-16
**Session:** 16:12 local
**Branch/Commit:** main / c751a8b
**Scope:** Continued v0.10.0 P1 release hardening after strict-arity slices. Removed the last exhaustive-dispatch probe skip by safely probing `input` and `exit` without triggering blocking I/O or process termination.

---

## What I Changed
- Updated exhaustive declared-builtin drift coverage in `src/interpreter/native_functions/mod.rs`.
- Replaced `skip_probe_names = ["input", "exit"]` with per-builtin safe probe args:
  - `input` probes with `Value::Int(1)` (shape error path)
  - `exit` probes with `Value::Str("non-numeric")` (shape error path)
- Kept all other builtins probed with `[]` to preserve broad dispatch drift coverage.
- Updated hardening docs to match implementation:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Exhaustive probe tests can still cover side-effecting builtins if you target deterministic contract-error paths.
  - **Symptom:** Previous tests skipped `input`/`exit`, leaving a residual hole in declared-builtin drift coverage.
  - **Root cause:** We treated side-effecting builtins as unprobeable, instead of distinguishing behavior paths.
  - **Fix:** Probe `input`/`exit` with intentionally wrong argument types to force immediate non-side-effect error returns.
  - **Prevention:** For side-effecting APIs, prefer safe invalid-shape probes over blanket skip lists.

- **Gotcha:** `apply_patch` can place edits in the wrong region when context is too broad and repeated.
  - **Symptom:** A patch inserted assertions into the wrong test block and broke function structure.
  - **Root cause:** The patch context matched a repeated structure near another test function.
  - **Fix:** Restore the file (`git restore`) and reapply a narrower patch anchored directly in the target function block.
  - **Prevention:** Use tighter context windows and verify placement in the intended function before continuing.

## Things I Learned
- Exhaustive drift guards are strongest when they include *all* declared builtins, even side-effecting ones, via safe probe inputs.
- A “skip list” is not always the right long-term state; contract-aware probe design can remove the skip while remaining deterministic.
- In this codebase, formatter spillover after `cargo fmt` is routine in large native-function files; restore unrelated files before committing focused slices.

## Debug Notes (Only if applicable)
- **Failing test / error:** Temporary parse/structure corruption in `src/interpreter/native_functions/mod.rs` after a misapplied patch.
- **Repro steps:** Apply a broad patch around repeated `#[test]` regions in `mod.rs`.
- **Breakpoints / logs used:** `read_file` around the affected line range and `git restore` recovery.
- **Final diagnosis:** Patch anchoring was ambiguous; file needed targeted re-patch with narrower context.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider extracting a helper in `src/interpreter/native_functions/mod.rs` for probe-argument selection in declared-builtin drift tests.
- [ ] Keep curated gotcha guidance synchronized whenever probe strategy changes (skip-list vs safe-probe).

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
