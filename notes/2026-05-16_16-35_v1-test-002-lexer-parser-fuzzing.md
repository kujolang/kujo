# Kujo Field Notes — V1-TEST-002 Lexer/Parser Fuzzing

**Date:** 2026-05-16
**Session:** 16:35 local
**Branch/Commit:** main / pending
**Scope:** Implemented `V1-TEST-002` by adding cargo-fuzz targets and corpus scaffolding for lexer/parser malformed-input fuzzing plus nightly CI smoke execution.

---

## What I Changed
- Added cargo-fuzz project in `fuzz/`:
  - `fuzz/Cargo.toml`
  - `fuzz/fuzz_targets/lexer.rs`
  - `fuzz/fuzz_targets/parser.rs`
  - `fuzz/.gitignore`
- Added seed corpus files:
  - `fuzz/corpus/lexer/*.kujo`
  - `fuzz/corpus/parser/*.kujo`
- Added nightly/manual fuzz smoke workflow:
  - `.github/workflows/fuzz-smoke.yml`
- Updated roadmap/changelog/README for `V1-TEST-002` completion and operational commands.

## Gotchas (Read This Next Time)
- **Gotcha:** Local `cargo +nightly check --manifest-path fuzz/Cargo.toml` can fail even after installing nightly and cargo-fuzz.
  - **Symptom:** `libfuzzer-sys` C++ compile failures such as missing `cassert`/`cstdint` headers.
  - **Root cause:** Host toolchain/SDK header availability and local compiler setup may not satisfy libFuzzer C++ build requirements.
  - **Fix:** Treat nightly CI smoke (`ubuntu-latest`) as authoritative for fuzz target compile/run status; local failure can be environmental.
  - **Prevention:** Keep fuzz execution in dedicated CI workflow with known-good Linux toolchain and explicit `cargo +nightly fuzz run ... -max_total_time=<N>` commands.

## Things I Learned
- Lossy UTF-8 conversion is the right bridge for byte-level fuzz input when lexer APIs accept `&str`.
- `parse_with_diagnostics()` is a better parser fuzz target entrypoint than strict parse success paths because it intentionally exercises recovery/error code paths.
- Dedicated fuzz workflow should remain outside main release gate to avoid making regular PR checks prohibitively slow.

## Debug Notes (Only if applicable)
- **Failing test / error:** `fatal error: 'cassert' file not found` while building `libfuzzer-sys` locally.
- **Repro steps:** `cargo +nightly check --manifest-path fuzz/Cargo.toml`.
- **Breakpoints / logs used:** Captured full toolchain output from `cargo +nightly check` after nightly install.
- **Final diagnosis:** Local environment toolchain headers unavailable for libFuzzer C++ build; CI nightly workflow is required as the stable compile/run signal.

## Follow-ups / TODO (For Future Agents)
- [ ] Add an internal helper script to run fuzz smoke locally with clearer prerequisite checks (clang/c++ headers, nightly, cargo-fuzz).
- [ ] Add parser/lexer crash reproduction automation if CI finds a fuzzing crash artifact.

## Links / References
- Files touched:
  - `fuzz/Cargo.toml`
  - `fuzz/fuzz_targets/lexer.rs`
  - `fuzz/fuzz_targets/parser.rs`
  - `fuzz/corpus/lexer/basic_valid.kujo`
  - `fuzz/corpus/lexer/malformed_escape.kujo`
  - `fuzz/corpus/lexer/unclosed_string.kujo`
  - `fuzz/corpus/parser/basic_control_flow.kujo`
  - `fuzz/corpus/parser/missing_delimiter.kujo`
  - `fuzz/corpus/parser/nested_literals.kujo`
  - `.github/workflows/fuzz-smoke.yml`
  - `ROADMAP.md`
  - `CHANGELOG.md`
  - `README.md`
- Related docs:
  - `ROADMAP.md`
  - `notes/GOTCHAS.md`
  - `notes/FIELD_NOTES_SYSTEM.md`
