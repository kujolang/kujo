# 2026-05-01 03:10 - v1 Artifact Gate And Match-Binding Parity

## Summary

Completed the remaining pre-v1.0.0 blockers:

- closed tag-style `match` binding capability gap across interpreter and VM paths
- added release-tag workflows for publishing prebuilt Linux/macOS binaries and checksums
- added post-publish artifact-only smoke validation workflow
- updated install/release docs and roadmap status markers for pre-tag v1 readiness

## Implementation Details

### Runtime/Compiler parity closure

- Added `OpCode::MatchCasePattern(String)` in `src/bytecode.rs`.
- Updated `Stmt::Match` lowering in `src/compiler.rs` to emit `MatchCasePattern` for case checks.
- Added VM `MatchCasePattern` execution in `src/vm.rs` with binding support for:
  - `Result::Ok(value)` / `Result::Err(value)`
  - `Option::Some(value)` / `Option::None`
  - tagged/enum/string/float fallback matching
- Updated compiler `Expr::Tag` lowering to map canonical constructors to structured opcodes:
  - `Result::Ok` -> `MakeOk`
  - `Result::Err` -> `MakeErr`
  - `Option::Some` -> `MakeSome`
  - `Option::None` -> `MakeNone`
- Aligned interpreter `match` handling in `src/interpreter/mod.rs` for full-tag and short-tag `Result`/`Option` patterns.
- Adjusted lexer/parser behavior so `Result`/`Option` constructor tags and enum-style match patterns parse consistently while preserving generic type-annotation parsing.
- Updated LSP diagnostics to keep malformed generic annotation diagnostics after parser panic paths were removed.
- Fixed parallel native API boundary test isolation by making temp roots process/counter unique.

### Release artifact gate automation

- Added `.github/workflows/release-binaries.yml`:
  - builds release binaries on Linux/macOS
  - validates `kujo --version`, `kujo run examples/hello.kujo`, `kujo lsp --help`
  - packages `kujo-<TAG>-<target>.tar.gz`
  - generates `.sha256` files
  - publishes artifacts and checksums on tag pushes
- Added `.github/workflows/release-published-artifact-smoke.yml`:
  - downloads published artifacts from GitHub Releases
  - verifies checksums
  - extracts and runs artifact binary directly (`--version`, `run`, `lsp --help`)
- Added `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` and updated `INSTALLATION.md` with standalone binary install flow.
- Updated `.github/scripts/validate-release-artifact.sh` to install from the already-built locked dependency set with `--offline`, so local artifact validation does not depend on registry access after the release build step.

## Validation

Executed:

- YAML parsing for `.github/workflows/release-binaries.yml` and `.github/workflows/release-published-artifact-smoke.yml`
- `cargo test --test parser_type_annotation_regressions`
- `cargo test --test vm_interpreter_parity_surfaces`
- `bash .github/scripts/check-release-state.sh`
- `cargo test --test cli_json_contracts --test package_module_workflow_integration --test stdlib_reference_contract --test native_api_security_boundaries`
- `bash .github/scripts/validate-release-artifact.sh`
- `cargo test`
- `cargo test --quiet`
- `rustfmt --edition 2021 --check` on touched Rust files

Result:

- workflow YAML parsed successfully
- parser regression suite passed with valid `Result<T, E>` / `Option<T>` annotation coverage preserved after lexer changes
- parity suite passed with tag-style match-binding scenario green on both interpreter and VM
- release-state guard passed for `Cargo.toml`, `README.md`, and `ROADMAP.md` version anchors
- release-critical CLI/package/stdlib/security suites passed
- local artifact install/checksum/tarball/extracted-binary validation passed
- full Rust test suite passed
- touched Rust files pass rustfmt check

Warning:

- `cargo install --locked --offline` reported locked dependency `core2 v0.4.0` is yanked in the crates.io registry. The locked offline release validation still passed; consider dependency refresh before or shortly after the v1 tag.

## Follow-through

For actual `v1.0.0` release tagging, complete the tag-time sign-off checklist and record final artifact URLs, checksum values, and workflow pass logs in the release evidence note.
