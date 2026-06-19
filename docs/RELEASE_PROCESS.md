# Kujo Release Process

Current active target: `1.0.0`
Current release state: pre-tag `1.0.0` release-candidate metadata; `Cargo.toml` is staged at `1.0.0`, but the final tag, publication, and binary artifact sign-off are not complete until dated release evidence exists.

This document is the canonical release and compatibility policy for Kujo.
It defines how maintainers cut releases, what compatibility guarantees apply, and which gates must pass before a tag is created.

## 1. Versioning Policy

Kujo follows semantic versioning-oriented release classes:

- `MAJOR` (`X.y.z`): compatibility reset with explicit migration guidance.
- `MINOR` (`x.Y.z`): additive language/runtime/tooling features and planned compatibility updates.
- `PATCH` (`x.y.Z`): compatibility-preserving fixes, security fixes, performance improvements, and documentation/process corrections.

Major-release policy note:

- Kujo's crate metadata may be staged at `1.0.0` during release-candidate validation.
- Staged `1.0.0` metadata does not by itself mean the final `v1.0.0` release has been tagged, published, or artifact-validated.
- Future major-version changes must include explicit compatibility and migration guidance after the final `v1.0.0` release is published.

## 2. Compatibility Policy

### 2.1 Language compatibility

- Previously valid syntax should remain valid within a patch line.
- Parser/runtime behavior changes that alter existing successful program output require explicit changelog notes.
- Intentional breaking language changes require a planned release-cycle callout and migration notes.

### 2.2 Standard library compatibility

- Stable builtin names and documented argument contracts should remain compatible within a patch line.
- Security hardening may tighten behavior (for example rejecting unsafe defaults) when required; such changes must be documented as compatibility-impacting fixes.
- Capability requirements for host-effect APIs are part of the compatibility contract and must be documented in `docs/STANDARD_LIBRARY.md` and `docs/NATIVE_API_SECURITY_POSTURE.md`.

### 2.3 Diagnostics and machine-readable contract stability

- Human-readable diagnostic text may improve over time.
- Diagnostic `code`, `subsystem`, and machine-readable JSON field shape are stability surfaces.
- CLI/LSP JSON field removals or type changes are breaking; additive optional fields are non-breaking.

### 2.4 CLI exit-code stability

Exit code classes are contract-stable for automation:

- `0`: success
- `1`: command/gate failure
- `2`: usage/argument parse failure
- `3`: lexer/parser diagnostics
- `4`: runtime semantic failure
- `5`: IO failure
- `6`: internal/tooling failure

### 2.5 Dependency lockfile determinism

- Kujo package workflows treat `kujo.lock` as the deterministic dependency snapshot derived from `kujo.toml`.
- `kujo package-install` regenerates lockfile state deterministically (stable ordering and schema metadata).
- `kujo package-install --frozen` is the verify mode and must fail when `kujo.lock` is missing or out of date relative to `kujo.toml`.
- Release candidates should include a lockfile verification pass for package workflow fixtures and examples that use manifests.

## 3. Required CI And Local Gates

The release gate is enforced by `scripts/release_gate.sh` and CI workflows.

Canonical full gate command:

```bash
bash scripts/release_gate.sh --full
```

Release-candidate readiness gate command:

```bash
bash scripts/release_candidate_gate.sh --full
```

Fast smoke gate command:

```bash
bash scripts/release_gate.sh --minimal
```

Roadmap-only readiness precheck:

```bash
bash scripts/release_candidate_gate.sh --roadmap-only
```

Full gate command order:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`
4. Focused integration suites:
   - `cargo test --test native_api_security_boundaries`
   - `cargo test --test package_module_workflow_integration`
   - `cargo test --test vm_interpreter_parity_surfaces`
   - optional `cargo test --test serve_command_integration` when `KUJO_ENABLE_SOCKET_TESTS=1`
5. Kujo self-test fixtures: `cargo run -- test`
6. Optional tools when installed: `cargo audit`, `cargo deny check`

Optional benchmark smoke in full mode:

```bash
KUJO_RELEASE_GATE_RUN_BENCH=1 bash scripts/release_gate.sh --full
```

## 4. Release Candidate (RC) Process

Before tagging `1.0.0-rc` or `1.0.0`:

1. Confirm all P0 and P1 roadmap items are complete in `ROADMAP.md`.
2. Confirm every deferred P2 item is explicitly documented.
3. Run full release gate and required integration suites.
4. Validate release-state consistency checks.
5. Record release evidence in `notes/` with exact commands and outcomes.

Recommended RC staging flow:

1. Branch from `main` (for example `release/1.0.0-rc1`).
2. Freeze scope to release blockers only.
3. Re-run full gates after each blocker fix.
4. Cut RC tag only from a clean tree.
5. Promote RC to final tag only after all gates remain green.

## 5. Changelog Format Policy

`CHANGELOG.md` must follow Keep a Changelog structure with concrete user impact.

Required section categories (when applicable):

- `Added`
- `Changed`
- `Fixed`
- `Security`
- `Performance`
- `Removed`

Release-entry template:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Fixed
- ...

### Security
- ...

### Performance
- ...
```

Rules:

- Keep entries specific and user-facing.
- Call out compatibility-impacting changes explicitly.
- Include migration guidance when behavior changes could break automation or scripts.

## 6. Pre-Release Checklist (Before Version Bump)

1. Confirm local repository state.

```bash
git status --short
git branch --show-current
```

2. Run release gates.

```bash
bash scripts/release_gate.sh --full
bash .github/scripts/check-release-state.sh
```

3. Validate editor extension baseline when cycle scope includes editor/tooling updates.

```bash
cd tools/vscode-kujo-extension
npm ci
npm run check
cd ../..
```

4. Confirm roadmap and artifact checklist status:

- `ROADMAP.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`

5. Confirm release evidence notes exist and are up to date under `notes/`.

## 7. Version Bump And Documentation Sync

1. Set `[package].version` in `Cargo.toml`.
2. Create/complete the target release section in `CHANGELOG.md`.
3. Update release-status strings in:
   - `README.md`
   - `ROADMAP.md`
4. Re-run release-state guard:

```bash
bash .github/scripts/check-release-state.sh
```

## 8. Tagging And Publication Order

1. Create release commit.

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md README.md ROADMAP.md docs/RELEASE_PROCESS.md
git commit -m ":rocket: RELEASE: vX.Y.Z"
git push origin main
```

2. Create and push annotated tag.

```bash
git tag -a vX.Y.Z -m "Kujo vX.Y.Z"
git push origin vX.Y.Z
```

3. Publish crate (when applicable):

```bash
cargo publish --dry-run
cargo publish
```

4. Publish GitHub release artifacts (Linux/macOS/Windows binaries + checksums) via release workflows.

5. Run published-artifact smoke validation and record outcomes.

## 9. Dry-Run Workflow (No Tag / No Publish)

Use before high-risk releases:

1. Create dry-run branch.
2. Execute full gates.
3. Apply version/doc edits as rehearsal.
4. Run checks again.
5. Create dry-run commit.
6. Record commands/results in `notes/`.
7. Discard or merge dry-run branch after review.

Dry-run success criteria:

- no undocumented manual steps
- deterministic command outcomes
- release-state guard passes

## 10. Required Release Evidence

Every release (including RC) needs a dated note under `notes/` that includes:

- execution context (local/CI, host details)
- exact commands run
- pass/fail status per command
- warnings/exceptions and rationale
- explicit sign-off statement for release readiness

Do not mark release checklist items complete without evidence.
