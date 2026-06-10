# Kujo Field Notes: Named Nested Closure Capture Parity

**Date:** 2026-05-12
**Session:** 23:23 local
**Roadmap Item:** NO-ROADMAP (named nested closure capture and mutation parity)
**Priority/Severity:** P1 / High
**Branch/Commit(s):** main / 163b2ed (working tree changes, uncommitted)
**Scope:** Fixed a runtime/compiler parity gap where nested named functions could fail to mutate captured outer bindings in VM mode. Added a parity regression test and verified the full VM/interpreter parity suite remains green.

---

## Outcome
- **Status:** complete
- **Roadmap updated:** no
- **Changelog updated:** yes (`CHANGELOG.md`, Unreleased -> Changed)
- **README/docs updated:** yes (`notes/GOTCHAS.md`, `notes/README.md`)
- **Behavior changed:** yes
- **Semantics changed:** yes

## What I Changed
- Updated interpreter named function-definition behavior in `src/interpreter/mod.rs` (`Stmt::FuncDef`) to capture lexical environment for nested scopes (`self.env.scopes.len() > 1`) for both `Value::Function` and `Value::AsyncFunction`.
- Fixed compiler free-variable analysis in `src/compiler.rs` (`find_free_variables` -> `collect_stmt_vars`) so `Stmt::Assign` identifier targets are treated as variable usage (`used.insert(name.clone())`) rather than local definition.
- Added parity regression test `vm_and_interpreter_match_named_nested_capture_mutation` in `tests/vm_interpreter_parity_surfaces.rs`.
- Added changelog entry under `CHANGELOG.md` Unreleased/Changed describing interpreter + compiler closure-capture parity fix.
- Updated curated gotcha in `notes/GOTCHAS.md` to replace outdated rule that named nested functions are non-closure expressions.
- Added session index entry in `notes/README.md`.

## Tests Run
- `cargo test vm_and_interpreter_match_named_nested_capture_mutation vm_and_interpreter_match_successful_captured_map_update --test vm_interpreter_parity_surfaces` — fail
- `cargo test vm_and_interpreter_match_named_nested_capture_mutation --test vm_interpreter_parity_surfaces` — pass
- `cargo test vm_and_interpreter_match_successful_captured_map_update --test vm_interpreter_parity_surfaces` — pass
- `cargo test --test vm_interpreter_parity_surfaces` — pass

## Gotchas (Read This Next Time)
- **Gotcha:** Fixing interpreter capture alone is not enough for named nested function mutation parity.
  - **Symptom:** New parity test failed in VM path with `Undefined variable: count`.
  - **Root cause:** Compiler free-variable analysis treated assignment targets (`count := ...`) as local definitions, so the closure upvalue list omitted mutated captured variables.
  - **Fix:** Treat identifier assignment targets as usage during free-variable collection.
  - **Prevention:** For closure work, always run both a named nested mutation test and the full parity suite.
- **Gotcha:** `cargo test` accepts only one positional test filter.
  - **Symptom:** Command with two positional test names failed immediately.
  - **Root cause:** Cargo CLI contract allows only one positional test filter.
  - **Fix:** Run tests separately or use one filter plus `-- --nocapture` options.
  - **Prevention:** Keep test command lines minimal and one-filter-per-invocation.

## Things I Learned
- Named nested functions in Kujo now need to be treated as closure surfaces for parity, not only anonymous function expressions.
- Closure-capture parity is a two-part invariant: interpreter capture behavior plus compiler upvalue detection.
- Assignment targets can represent outer-binding mutation and must not be assumed local declarations in capture analysis.

## Debug Notes (Only if applicable)
- **Failing test / error:** `vm execution failed: Some("Undefined variable: count")`
- **Repro steps:** Run `cargo test vm_and_interpreter_match_named_nested_capture_mutation --test vm_interpreter_parity_surfaces` before the compiler free-variable fix.
- **Breakpoints / logs used:** Source inspection in `src/compiler.rs` (`find_free_variables`, `collect_stmt_vars`) and `src/vm.rs` (`OpCode::MakeClosure`, `OpCode::LoadVar`, `OpCode::StoreVar`).
- **Final diagnosis:** Mutated identifier assignment target was incorrectly excluded from closure free-variable capture list.

## Assumptions I Almost Made (Only if applicable)
- Interpreter capture update would automatically guarantee VM parity for named nested closure mutation.
- Existing closure-expression parity tests were sufficient to cover named nested function semantics.

## Follow-ups / TODO
- [ ] Add an explicit async named nested closure-capture parity test in `tests/vm_interpreter_parity_surfaces.rs`.
- [ ] Update `mcp` docs note in `mcp.kujo`/README to avoid overstating current closure-mutation limitation after Kujo runtime parity fix.
- [ ] Evaluate whether a dedicated roadmap item is needed for broader closure semantic contract documentation beyond this bug fix.

## Links / References
- Roadmap:
  - `ROADMAP.md` — NO-ROADMAP (work not tracked under a current item)
- Files touched:
  - `src/interpreter/mod.rs`
  - `src/compiler.rs`
  - `tests/vm_interpreter_parity_surfaces.rs`
  - `CHANGELOG.md`
  - `notes/GOTCHAS.md`
  - `notes/README.md`
- Tests:
  - `tests/vm_interpreter_parity_surfaces.rs`
- Related docs:
  - `README.md`
  - `CHANGELOG.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
