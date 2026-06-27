# Kujo v1.0 Official Release Checklist

Status: active canonical operational checklist before official `v1.0.0` publication
Last updated: 2026-06-27

Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.

Use this file as the one human-facing checklist to run before the official release. Historical planning docs remain in the repo for audit evidence, but this file is the current operational path.

## Current Release Answer

Kujo's non-release implementation work is ready for final release review when the commands below pass on current `main`. The remaining blocker is release-flight artifact publication and sign-off, which requires the explicit `UNBLOCK_V1_RELEASE` directive in `docs/RELEASE_PROCESS.md`.

## Active Status Table

| Area | Status | Evidence |
| --- | --- | --- |
| Core language/runtime implementation | Ready for release review | `cargo test`, `cargo run -- test --runtime vm`, `cargo run -- test --runtime dual` |
| Core AI-native mechanisms | Ready for release review | `docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md`, `bash scripts/enterprise_verify.sh --full` |
| Security and host-effect boundaries | Ready for release review | `cargo test --test native_api_security_boundaries`, `docs/NATIVE_API_SECURITY_POSTURE.md`, `docs/SECURE_AI_SCRIPTING.md` |
| Generated release evidence | Ready after refresh | `bash scripts/generate_v1_code_todo_triage.sh`, `bash scripts/generate_pre_v1_unresolved_inventory.sh`, `bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict` |
| VM fixture coverage | Ready for release review | `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` reports `P0 runtime-parity-bug: 0` and `vm_matches_snapshot: 144/144` |
| Deferred runtime internals | Accepted for v1.0 | `docs/V1_SCOPE.md` lists explicit non-silent deferrals; current behavior is contract-locked by release gates |
| Legacy interpreter-only drift | Accepted post-v1 debt | Generated mismatch inventory classifies residual interpreter-only rows as `P2 intentional-divergence`; default VM output matches release snapshots |
| Tag-time release artifacts | Blocked until release directive | `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` remains the artifact sign-off record |

## Pre-Release Verification Order

Run these immediately before release review:

```bash
git status --short --branch
bash scripts/generate_v1_code_todo_triage.sh
bash scripts/generate_pre_v1_unresolved_inventory.sh
bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- test --runtime vm
cargo run -- test --runtime dual
bash scripts/enterprise_verify.sh --full
bash scripts/release_gate.sh --full
```

For low-contention final release-candidate evidence, also run:

```bash
KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full
```

## Release-Flight Step

Only after release owners provide `UNBLOCK_V1_RELEASE`, execute the tag-time publication/sign-off flow in:

- `docs/RELEASE_PROCESS.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`

The sign-off must record release URLs, Linux/macOS/Windows assets, per-asset SHA-256 files, consolidated `checksums.txt`, published-artifact smoke workflow status, and a dated `notes/` evidence file.

## Superseded Planning Docs

The following files are retained as historical planning/evidence, not as competing active checklists:

- `docs/AI_CENTRIC_GAP_ANALYSIS.md`
- `docs/ENTERPRISE_READINESS_NEXT_SESSION_2026-06-20.md`
- `docs/ENTERPRISE_AI_NATIVE_POLISH_NEXT_SESSION_2026-06-27.md`
- `docs/V1_0_FINAL_REVIEW_BLOCKERS_2026-06-20.md`
- `docs/V1_0_REMAINING_NON_RELEASE_WORK_CHECKLIST.md`
- `docs/V1_0_NEXT_SESSION_ACTIONS_2026-06-20.md`
