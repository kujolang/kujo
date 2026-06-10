# Kujo Field Notes — V1-PAR-003 precedence and assignment operator lock

**Date:** 2026-05-07
**Session:** 08:12 local
**Branch/Commit:** main / 6ed6705
**Scope:** Implemented roadmap item V1-PAR-003 by locking parser precedence/associativity behavior with regression tests, adding compound assignment parsing support, and updating spec/docs contracts.

---

## What I Changed
- Added `tests/parser_precedence.rs` with AST-shape and runtime precedence/associativity coverage.
- Split parser equality/comparison precedence tiers in `src/parser.rs`:
  - equality: `==`, `!=`
  - comparison: `<`, `<=`, `>`, `>=`
- Added parser support for compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`) by lowering to regular assignment with a binary expression RHS.
- Added deterministic chained-assignment parser diagnostic (`a := b := 1` rejected).
- Added lexer support for tokenizing compound assignment operators in `src/lexer.rs`.
- Added lexer regression test `tokenizes_compound_assignment_operators`.
- Updated docs in `docs/LANGUAGE_SPEC.md`, `README.md`, `CHANGELOG.md`, and marked `V1-PAR-003` complete in `ROADMAP.md`.

## Gotchas (Read This Next Time)
- **Gotcha:** Splitting equality and comparison precedence can silently narrow expression support in parser helper callsites.
  - **Symptom:** Literal/member helper parses start rejecting previously accepted equality expressions after precedence refactors.
  - **Root cause:** `parse_comparison()` was reused in array/dict/struct-literal helper paths and previously included equality operators.
  - **Fix:** Move those helper callsites to `parse_equality()` when preserving pre-existing expression coverage is required.
  - **Prevention:** After precedence-level refactors, run an explicit `rg` sweep for old parse-stage helper usage and re-evaluate each callsite contract.

## Things I Learned
- Compound assignment support required coordinated lexer+parser updates; parser-only changes are insufficient because `+=`/`-=`/etc were previously tokenized as separate operators.
- Statement-level assignment in Kujo is currently non-associative by design, so chained assignment should fail explicitly rather than degrading into partial-statement parse behavior.
- AST-shape tests are a good fit for precedence contracts because they keep behavior assertions precise without coupling to debug formatting.

## Debug Notes (Only if applicable)
- **Failing test / error:** `tests/parser_precedence.rs` initially failed on equality/comparison grouping and compound assignment parse errors (`Expected expression`).
- **Repro steps:** `cargo test --test parser_precedence`.
- **Breakpoints / logs used:** Focused test failures plus source inspection in `src/parser.rs` and `src/lexer.rs`.
- **Final diagnosis:** Parser grouped equality and comparison at the same precedence level, and lexer lacked compound assignment tokenization.

## Follow-ups / TODO (For Future Agents)
- [ ] Evaluate whether assignment-expression support (not just statement-level assignment) should be formalized or intentionally deferred for v1.
- [ ] Keep parser precedence table updates synchronized with future operator additions (`docs/LANGUAGE_SPEC.md` + `tests/parser_precedence.rs`).

## Links / References
- Files touched:
  - `src/parser.rs`
  - `src/lexer.rs`
  - `tests/parser_precedence.rs`
  - `docs/LANGUAGE_SPEC.md`
  - `README.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/LANGUAGE_SPEC.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
