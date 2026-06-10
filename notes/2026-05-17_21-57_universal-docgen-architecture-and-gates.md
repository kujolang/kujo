# Kujo Field Notes: Universal DocGen Architecture and Gate Hardening

**Date:** 2026-05-17
**Session:** 21:57 local
**Roadmap Item:** NO-ROADMAP (prompt-driven universal `kujo docgen` expansion)
**Priority/Severity:** P1 / High
**Branch/Commit(s):** `main` / not committed yet
**Scope:** Replaced Kujo-only doc generation internals with a universal adapter-based pipeline, expanded CLI/options/outputs, preserved legacy JSON contract fields, and added multi-language/security/determinism integration tests.

---

## Outcome
- **Status:** complete
- **Roadmap updated:** no
- **Changelog updated:** yes (`CHANGELOG.md` Unreleased Added)
- **README/docs updated:** yes (`README.md`, `docs/CLI_MACHINE_READABLE_CONTRACTS.md`, `docs/DOCGEN.md`)
- **Behavior changed:** yes
- **Semantics changed:** no

## What I Changed
- Refactored docgen internals into universal module tree under `src/docgen/`:
  - `src/docgen/core.rs`
  - `src/docgen/model.rs`
  - `src/docgen/discovery.rs`
  - `src/docgen/gaps.rs`
  - `src/docgen/render/{html.rs,markdown.rs,json.rs}`
  - `src/docgen/adapters/{mod.rs,common.rs,kujo.rs,php.rs,python.rs,typescript.rs,javascript.rs,ruby.rs,go.rs,haskell.rs,zig.rs}`
- Added shared adapter trait (`DocLanguageAdapter`) and adapter registry with capability metadata output (`docgen-capabilities.json`).
- Implemented Kujo-first adapter with symbol extraction for functions/methods/structs/enums/enum variants/constants and `///` docs attachment plus placeholders.
- Implemented initial heuristic adapters for PHP/Python/TypeScript/JavaScript/Ruby/Go/Haskell/Zig to extract top-level symbols, basic type/class/method signatures, inline docs, source paths/lines, and gaps.
- Added secure discovery constraints:
  - skip symlinks during tree walk
  - canonical root containment checks
  - deterministic sorted traversal
  - depth/file-count/file-size limits
- Added gap + AI task artifacts:
  - `docgen-gaps.json`
  - optional `docgen-ai-tasks.md` via `--emit-ai-tasks`
- Added strict gate computation in docgen core and CLI exit behavior for:
  - `--fail-on-undocumented`
  - `--fail-on-broken-links`
  - `--fail-on-warnings`
- Expanded `kujo docgen` CLI in `src/main.rs` with:
  - path input (`file` -> `path`)
  - `--format`
  - `--language`
  - `--languages`
  - `--emit-ai-tasks`
  - `--search-index`
  - `--source-links`
  - `--public-only`
  - `--include-private`
  - strict gate flags above
- Preserved backward-compatible JSON fields for existing automation (`command`, `file`, `output_dir`, `module_doc_path`, `builtin_doc_path`, `item_count`) while adding new fields.
- Kept compatibility wrapper in `src/doc_generator.rs` delegating to new core to preserve old call sites/tests.
- Added integration coverage in `tests/docgen_universal.rs` for:
  - documented/undocumented Kujo symbols
  - mixed-language repositories
  - deterministic output stability
  - symlink escape avoidance
  - no source execution guarantee
  - HTML escaping
  - strict gate failure reporting
  - adapter conformance smoke
  - large-repo smoke
  - output snapshot-style checks
- Updated exports in `src/lib.rs` and docs/changelog surfaces.

## Tests Run
- `cargo test docgen -- --nocapture` — pass
- `cargo test --test docgen_universal` — pass
- `cargo test docgen_json_contract_is_stable --test cli_json_contracts` — pass
- `cargo fmt` — pass (rustfmt emitted existing nightly-feature warnings only)

Full repository test suite was not run in this session because scope was concentrated on docgen architecture/contract surfaces and dedicated integration coverage.

## Gotchas (Read This Next Time)
- **Gotcha:** Legacy `docgen --json` fields are a compatibility contract, not optional convenience.
  - **Symptom:** Easy to break external consumers while adding new JSON fields.
  - **Root cause:** Existing tests assert legacy keys, and automation depends on them.
  - **Fix:** Keep old fields stable and additive-extend payload.
  - **Prevention:** Always run `cargo test docgen_json_contract_is_stable --test cli_json_contracts` after docgen JSON changes.

- **Gotcha:** Strict gate failures are computed in core but mapped to CLI exit at command layer.
  - **Symptom:** Core can report failures without process failure if CLI handling is skipped.
  - **Root cause:** `run()` returns summary metadata; exit semantics live in `src/main.rs`.
  - **Fix:** Explicitly exit non-zero in `Commands::Docgen` when `gate_failures` is non-empty.
  - **Prevention:** Treat gate policy as an end-to-end CLI contract, not just a core data contract.

## Things I Learned
- The safest migration path for a large feature here was “new module + legacy wrapper,” not in-place mutation of old `src/doc_generator.rs` behavior.
- Rule: keep discovery deterministic first (sorted dirs/files, bounded scans), then add feature depth; CI stability depends on ordering before rendering details.
- Rule: adapters can be heuristic initially if the output model is stable and security constraints are strict (no execution/import/build).
- Rule: for high-change CLI payloads, additive JSON extension is fine; replacing/removing stable keys is not.

## Debug Notes (Only if applicable)
- **Failing test / error:** `error[E0658]: let expressions in this position are unstable` in `src/docgen/discovery.rs`
- **Repro steps:** run `cargo test docgen -- --nocapture`
- **Breakpoints / logs used:** compiler error output only
- **Final diagnosis:** used Rust `if let ... && ...` chain syntax not accepted in this toolchain position; rewrote to nested `if`.

- **Failing test / error:** `no variant or associated item named 'CommandFailure' found for enum CliExitCode`
- **Repro steps:** run focused docgen tests after CLI gate-wiring edits
- **Breakpoints / logs used:** compiler error output + `src/main.rs` enum inspection
- **Final diagnosis:** `CliExitCode` intentionally has fixed variants and no `CommandFailure`; use explicit exit code `1` for unmet gates.

## Assumptions I Almost Made (Only if applicable)
- I almost assumed there was a reusable `CliExitCode::CommandFailure` variant for unmet gate behavior. That assumption was wrong in this repo; generic gate failures should exit with code `1` directly.

## Follow-ups / TODO
- [ ] Add/update a dedicated roadmap item for universal docgen staging (current work is prompt-driven and not tied to an explicit `ROADMAP.md` ID).
- [ ] Add deeper parser-quality adapters (AST-backed) over time for non-Kujo languages to reduce regex heuristics and false positives.
- [ ] Run a full `cargo test` sweep before merge/commit to catch any non-docgen regressions.

## Links / References
- Roadmap:
  - `ROADMAP.md` — NO-ROADMAP in this session (prompt-driven work)
- Files touched:
  - `src/main.rs`
  - `src/lib.rs`
  - `src/doc_generator.rs`
  - `src/docgen/mod.rs`
  - `src/docgen/core.rs`
  - `src/docgen/model.rs`
  - `src/docgen/discovery.rs`
  - `src/docgen/gaps.rs`
  - `src/docgen/render/html.rs`
  - `src/docgen/render/markdown.rs`
  - `src/docgen/render/json.rs`
  - `src/docgen/adapters/mod.rs`
  - `src/docgen/adapters/common.rs`
  - `src/docgen/adapters/kujo.rs`
  - `src/docgen/adapters/php.rs`
  - `src/docgen/adapters/python.rs`
  - `src/docgen/adapters/typescript.rs`
  - `src/docgen/adapters/javascript.rs`
  - `src/docgen/adapters/ruby.rs`
  - `src/docgen/adapters/go.rs`
  - `src/docgen/adapters/haskell.rs`
  - `src/docgen/adapters/zig.rs`
  - `tests/docgen_universal.rs`
  - `docs/DOCGEN.md`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `README.md`
  - `CHANGELOG.md`
- Tests:
  - `tests/docgen_universal.rs`
  - `tests/cli_json_contracts.rs`
- Related docs:
  - `README.md`
  - `CHANGELOG.md`
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `docs/DOCGEN.md`
