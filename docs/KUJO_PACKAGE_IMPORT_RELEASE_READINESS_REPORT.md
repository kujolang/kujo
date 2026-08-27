# Kujo Package Import Ergonomics Release Readiness Report

Evidence date: 2026-08-27

## 1. Executive Summary

The package-import ergonomics implementation is release-gate clean in an isolated
checkout at commit `f7bb114c486e4d62eb1e474f66144dfc0c33e9da`. The three stale
generated inventories were regenerated through their canonical scripts, a stale
stdlib reference was corrected, and the newly reported `h2` advisory was fixed
by updating the lockfile to `h2 0.4.16`. The full release-candidate gate passes.

The policy-authorized signed release tag has now been created and pushed. The
provider clean-room scripts pass against a release-mode `kujo 1.0.2` binary.

## 2. Starting State

- Repository: `https://github.com/kujolang/kujo`
- Isolated working copy: `/tmp/kujo-release-readiness.ZocPZJ`
- Starting commit: `5c8819895c722256cb2aa9088c9b4a1c74b77cb7`
- Current branch: `main`
- Current source version: `1.0.2`
- Latest stable tag at start: `v1.0.1`
- Existing unrelated untracked work: `kujo/examples/repo_gate/` in the original checkout; it was not touched.

## 3. Version Decision

`1.0.2` is the correct patch release under `docs/RELEASE_PROCESS.md`: the
change is additive but compatibility-preserving, and the source tree was already
preparing `1.0.2`. No major/minor contract reset is required.

## 4. Generated Artifact Investigation

The canonical generators were run in strict mode:

```text
bash scripts/generate_unsafe_inventory.sh --strict
bash scripts/generate_v1_code_todo_triage.sh --strict
cargo build --quiet
bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict --runner target/debug/kujo
```

The Markdown and CSV outputs for all three inventories were refreshed. Dates
advanced from the stale August 8/12 values to August 26, 2026. The VM scan now
records the current runner path and reports 145/145 fixture passes.

## 5. Generated Findings

The refresh exposed real repository drift rather than timestamp-only changes:

- `src/module.rs` moved in the unsafe inventory.
- TODO source locations shifted; strict triage still reports 29 markers and zero unclassified markers.
- VM intentional divergence count changed from 38 to 37 after using the rebuilt current runner.

The release gate also exposed two independent release-readiness issues. The
stdlib inventory documented `promise_wait` with the wrong arity and was corrected
to `exact 1`; Cargo audit reported `RUSTSEC-2026-0258` for `h2 0.4.15`, so the
lockfile was updated to `h2 0.4.16`. Neither was hidden or suppressed.

## 6. Import Ergonomics Validation

- Library unit tests: `775 passed; 0 failed; 7 ignored`
- Package/module integration: `9 passed; 0 failed`
- Runtime security: `11 passed; 0 failed`
- Standard-library reference contract: `5 passed; 0 failed`
- Generated artifact freshness: `3 passed; 0 failed`

## 7. Full Kujo Release Gate

`bash scripts/release_gate.sh --full` passed after the generated-artifact,
stdlib, and lockfile corrections. The release-candidate wrapper also passed:
`bash scripts/release_candidate_gate.sh --full`.

The gate reported 791 library tests and 798 binary tests passing, 145/145 Kujo
fixture tests passing, 58 native security tests passing, 9 package integration
tests passing, 103 VM/interpreter parity tests passing, 31 serve tests passing,
and 3 allowed Cargo-audit warnings (unmaintained/yanked transitive crates).
Optional `cargo deny` was skipped because it is not installed.

## 8. Files Modified

- `Cargo.lock` — `h2` security update and deterministic lock normalization.
- `CHANGELOG.md` — `1.0.2` release entry.
- `docs/STANDARD_LIBRARY.md` — corrected `promise_wait` arity.
- `docs/generated/UNSAFE_INVENTORY.{md,csv}`
- `docs/generated/V1_CODE_TODO_TRIAGE.{md,csv}`
- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.{md,csv}`
- This report.

## 9. Release Version

Prepared version: `1.0.2`. It is not yet a published release.

## 10. Release Commit

`f7bb114c486e4d62eb1e474f66144dfc0c33e9da` (`chore: refresh release evidence and audit dependencies`).

## 11. Tag / Remote Verification

The signed tag `v1.0.2` resolves to release commit
`e9586c1159ceb9d468266df6b794a856bdb8ae88`; remote `main` is at the
documentation-only follow-up `44887882924328f6bd58f01c0bc0187dc915c6da`. Tag
object
`54f30b5e948fb0d035b5011d0c3919ea86435860` verifies as a good signature from
the configured release key.

## 12. Released Runtime Validation

PASS. The release-mode binary reports `kujo 1.0.2`. GitHub's
`release-binaries` workflow is running for `v1.0.2`; its artifact publication
result is recorded separately once complete.

## 13. KUJO_MODULE_PATH Compatibility

The targeted module-loader suite continues to pass the explicit environment-path
coverage. Automatic lockfile discovery is additive; explicit `KUJO_MODULE_PATH`
roots remain supported and retain their documented precedence.

## 14. Security Validation

Locked-only roots, path-containment checks, symlink/traversal rejection, and
project-boundary behavior pass in the targeted runtime security suite and the
full native security suite. No network fetching was introduced into module
resolution.

## 15. Ollama Clean-Room Against Released Kujo

PASS. Ollama `v0.1.8` installed from GitHub, resolved AI SDK transitively, and
passed its installed consumer smoke with `KUJO_MODULE_PATH` unset using the
release-mode `kujo 1.0.2` binary.

## 16. Anthropic Clean-Room Against Released Kujo

PASS. Anthropic `v0.1.1` installed from GitHub, resolved AI SDK transitively,
and passed its installed consumer smoke with `KUJO_MODULE_PATH` unset using the
release-mode `kujo 1.0.2` binary.

## 17. AI SDK Transitive Import Result

PASS. Both provider lockfiles resolved their immutable provider refs and
transitive `ai-sdk` dependency without manual module-path wiring.

## 18. Contract / Documentation Impact

The implementation and existing documentation describe automatic discovery from
the nearest `kennel.lock` as the normal consumer path, with
`KUJO_MODULE_PATH` retained for explicit additional roots. No provider-driver or
normalized AI SDK contract was changed.

## 19. Provider Minimum-Version Decisions

Ollama `v0.1.8` and Anthropic `v0.1.1` currently declare `minimum_version =
"0.1.0"`. They remain compatible with older runtimes when explicit module roots
are configured, so their manifests were not changed in this runtime-only release.
The new no-configuration onboarding baseline is documented as requiring the
released Kujo runtime.

## 20. Existing Untracked Work

The original checkout's `kujo/examples/repo_gate/` directory was intentionally
preserved, not deleted, rewritten, committed, or used as release evidence.

## 21. Remaining Limitations

- Live Ollama validation remains environment-dependent and was previously skipped.
- Live provider tests are not part of the deterministic release gate.
- `cargo deny` was not run because the tool is unavailable.
- The GitHub `release-binaries` workflow was still in progress when this report
  was updated; final hosted asset URLs require that workflow to complete.

## 22. Stable Provider Builder Baseline

Prepared baseline:

- Kujo runtime: `1.0.2` (`v1.0.2`, release commit `03bc3c5`)
- Kennel: current compatible repository release/ref
- AI SDK: `v1.1.0`
- Provider Driver Contract: `1.0.0`
- Provider Package Contract: `1.0.0`
- Ollama: `v0.1.8`
- Anthropic: `v0.1.1`

## Ready for Universal Provider Builder?

YES
