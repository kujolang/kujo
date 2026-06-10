# Kujo Field Notes — Release hardening dispatch gap slices

**Date:** 2026-02-15
**Session:** 16:17 local
**Branch/Commit:** main / eeb3f1c
**Scope:** Implemented multiple v0.10 release-hardening follow-through slices to reduce builtin declaration/dispatch drift in modular native handlers. Added contract tests, tightened exhaustive dispatch drift guard expectations, and kept docs/roadmap/changelog in sync per slice.

---

## What I Changed
- Added `par_each` hardening coverage in `src/interpreter/native_functions/mod.rs` and `src/interpreter/native_functions/async_ops.rs`:
  - dispatch regression coverage includes `par_each`
  - argument/error-shape parity checks for `par_each` vs `parallel_map`
- Added exhaustive declared-builtin dispatch drift guard in `src/interpreter/native_functions/mod.rs`:
  - probes `Interpreter::get_builtin_names()` against modular dispatch
  - compares unknown-dispatch results against an explicit known-gap list
  - skips side-effecting probes: `input`, `exit`, `sleep`, `execute`
- Migrated system env/args APIs into modular handlers in `src/interpreter/native_functions/system.rs`:
  - `env`, `env_or`, `env_int`, `env_float`, `env_bool`, `env_required`, `env_set`, `env_list`, `args`, `arg_parser`
  - added targeted module tests for env behavior + ArgParser shape
- Migrated data-format/encoding APIs into modular handlers in `src/interpreter/native_functions/json.rs`:
  - `parse_json`/`to_json`, TOML/YAML/CSV parse+serialize, Base64 encode/decode
  - added targeted tests (round-trip + argument-shape validation)
- Migrated regex APIs into modular handlers in `src/interpreter/native_functions/strings.rs`:
  - `regex_match`, `regex_find_all`, `regex_replace`, `regex_split`
  - added targeted tests (behavior + argument-shape validation)
- Repeatedly tightened the known-gap drift list in `src/interpreter/native_functions/mod.rs` as each slice was migrated.
- Updated `CHANGELOG.md`, `ROADMAP.md`, and `README.md` for each completed hardening slice.

## Gotchas (Read This Next Time)
- **Gotcha:** A full builtin dispatch probe can accidentally trigger process-level side effects.
  - **Symptom:** Running exhaustive dispatch tests can hang, sleep, prompt for input, execute shell commands, or terminate process.
  - **Root cause:** Some declared builtins are intentionally side-effecting (`input`, `exit`, `sleep`, `execute`) and are unsafe to probe naively.
  - **Fix:** Add a dedicated skip list in the exhaustive drift-guard test for side-effecting builtins.
  - **Prevention:** Treat exhaustive probe tests as *safe API coverage*, not *invoke every builtin blindly*.

- **Gotcha:** Exhaustive dispatch parity cannot be asserted as “zero gaps” while migration is in-flight.
  - **Symptom:** Initial exhaustive drift test failed with a large list of unknown-dispatch builtins.
  - **Root cause:** `get_builtin_names()` exposes legacy-complete API surface while modular dispatch migration is still incremental.
  - **Fix:** Assert against an explicit expected known-gap list and shrink it slice-by-slice.
  - **Prevention:** Keep the known-gap list deterministic, sorted in stable order, and updated in the same commit as each migrated slice.

- **Gotcha:** Legacy behavior shape must be mirrored exactly during modular extraction.
  - **Symptom:** Drift-guard and behavior tests fail even when handler names exist.
  - **Root cause:** Return/value/error shapes are part of API contract (for example, `ErrorObject` vs `Error`, `Struct` field names for `ArgParser`).
  - **Fix:** Copy semantics from `src/interpreter/legacy_full.rs` and add module-local tests for shape contracts.
  - **Prevention:** Always pair extraction with targeted tests for both success and validation paths.

## Things I Learned
- Modular hardening is fastest when done as a repeated loop: migrate one subsystem, tighten drift list, run focused tests, then full suite.
- The exhaustive drift guard is most valuable as a **debt ledger**: it should fail on *unexpected* drift, not on *known* unmigrated legacy surfaces.
- “Just registration parity” is insufficient; runtime dispatch parity requires behavior-level tests in the destination module.
- Keeping docs updates per slice made roadmap state reliable during rapid incremental migration.

## Debug Notes (Only if applicable)
- **Failing test / error:** `Declared builtins missing dispatcher coverage: [...]` from `test_release_hardening_builtin_dispatch_coverage_for_declared_builtins`.
- **Repro steps:**
  - `cargo test test_release_hardening_builtin_dispatch_coverage_for_declared_builtins`
- **Breakpoints / logs used:**
  - Compared failing builtin names against `Interpreter::get_builtin_names()` and module handlers in `src/interpreter/native_functions/*.rs`.
- **Final diagnosis:**
  - Drift was real and broad; converted test to explicit known-gap contract, then reduced the list by migrating system/data-format/regex slices.

## Follow-ups / TODO (For Future Agents)
- [ ] Migrate `http_*` and response helper APIs into modular dispatch and remove them from known-gap list.
- [ ] Migrate DB pool/transaction APIs into modular dispatch and remove them from known-gap list.
- [ ] Migrate process/network/crypto/archive/API leftovers and continue shrinking known-gap list toward zero.
- [ ] Revisit exhaustive dispatch guard once high-risk side-effecting builtins have safe test shims.

## Links / References
- Files touched:
  - `src/interpreter/native_functions/mod.rs`
  - `src/interpreter/native_functions/async_ops.rs`
  - `src/interpreter/native_functions/system.rs`
  - `src/interpreter/native_functions/json.rs`
  - `src/interpreter/native_functions/strings.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/GOTCHAS.md`
  - `notes/README.md`
  - `ROADMAP.md`
  - `CHANGELOG.md`
