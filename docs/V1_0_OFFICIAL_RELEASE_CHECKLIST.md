# Kujo v1.0 Official Release Checklist

Status: completed `v1.0.0` launch checklist
Last updated: 2026-08-08

Release boundary: Kujo `v1.0.1` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

This file preserves the human-facing verification path used for the official release. Historical planning docs remain in the repository as audit evidence.

## Current Release Answer

Kujo's v1 implementation and local release gates passed before the `v1.0.0` tag. Published asset and smoke-test evidence is recorded in `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` and the dated launch note.

## Active Status Table

| Area | Status | Evidence |
| --- | --- | --- |
| Core language/runtime implementation | Verified for v1.0.0 | `cargo test`, `cargo run -- test --runtime vm`, `cargo run -- test --runtime dual` |
| Core AI-native mechanisms | Verified for v1.0.0 | `docs/AI_NATIVE_ENTERPRISE_RELEASE_EVIDENCE.md`, `bash scripts/enterprise_verify.sh --full` |
| Security and host-effect boundaries | Verified for v1.0.0 | `cargo test --test native_api_security_boundaries`, `docs/NATIVE_API_SECURITY_POSTURE.md`, `docs/SECURE_AI_SCRIPTING.md` |
| Generated release evidence | Refreshed for launch | `bash scripts/generate_v1_code_todo_triage.sh`, `bash scripts/generate_pre_v1_unresolved_inventory.sh`, `bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict` |
| VM fixture coverage | Verified for v1.0.0 | `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` reports `P0 runtime-parity-bug: 0` and `vm_matches_snapshot: 145/145` |
| Deferred runtime internals | Accepted for v1.0 | `docs/V1_SCOPE.md` lists explicit non-silent deferrals; current behavior is contract-locked by release gates |
| Legacy interpreter-only drift | Accepted post-v1 debt | Generated mismatch inventory classifies residual interpreter-only rows as `P2 intentional-divergence`; default VM output matches release snapshots |
| Tag-time release artifacts | Published and verified | `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` is the artifact sign-off record |

## Release Verification Order

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

For low-contention final release evidence, also run:

```bash
KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full
```

## Release-Flight Record

Release owners provided `UNBLOCK_V1_RELEASE` before executing the tag-time publication/sign-off flow in:

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
