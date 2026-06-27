# AI-Native Enterprise Release Evidence

Status: active release-candidate evidence index
Last updated: 2026-06-27

This index is the current operator-facing evidence map for Kujo's AI-native release-candidate branch. It does not replace `ROADMAP.md` or the final tag checklist; it gives reviewers one place to find the commands, artifacts, and readiness boundaries that support the current product posture.

## Readiness Boundary

Kujo has the core AI-native mechanisms described in `docs/AI_RUNTIME.md`, but it is not yet a final-tagged, universally enterprise-ready release. The canonical release boundary remains:

> Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.

## Latest Local Verification Matrix

Run the full enterprise verification matrix before claiming a release-candidate build is ready for PR or tag review:

```bash
bash scripts/enterprise_verify.sh --full
```

The full wrapper covers:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo test --test docs_examples`
- `cargo test --test readme_contracts`
- `cargo test --test cli_contracts`
- `cargo test --test cli_json_contracts`
- `cargo test --test diagnostics_golden`
- `cargo run -- test --runtime vm`
- `cargo run -- test --runtime dual`
- `bash scripts/release_gate.sh --full`
- `cargo run -- check examples/ai_enterprise_replay_showcase.kujo`
- strict replay execution of `examples/ai_enterprise_replay_showcase.kujo`
- `cargo test --test ai_replay_hermeticity_contract`
- final `bash scripts/repo_hygiene_audit.sh`

For quick iteration on docs, security posture, replay showcase, and README contracts:

```bash
bash scripts/enterprise_verify.sh --minimal
```

Use dry-run mode when updating CI docs or reviewing command coverage:

```bash
bash scripts/enterprise_verify.sh --full --dry-run
```

## AI Replay And No-Live-Socket Evidence

AI examples and tests that claim determinism should run with strict replay:

```bash
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes \
KUJO_AI_REPLAY_MODE=strict \
cargo run -- run examples/ai_enterprise_replay_showcase.kujo
```

Strict replay resolves cassettes before destination-policy checks or HTTP client creation. Replay misses return deterministic `kind:"replay_miss"` failures instead of falling through to the network.

The regression suite `cargo test --test ai_replay_hermeticity_contract` verifies this by pointing a missing cassette at a loopback endpoint and asserting the result is a replay miss rather than a socket failure.

Do not set `KUJO_AI_REPLAY_MODE=fallthrough` in CI for deterministic AI tests. Fallthrough is only for intentionally recording or refreshing fixtures.

## Generated Evidence

Current generated evidence lives under `docs/generated/`:

- `docs/generated/PRE_V1_UNRESOLVED_INVENTORY.md`
- `docs/generated/UNSAFE_INVENTORY.md`
- `docs/generated/V1_CODE_TODO_TRIAGE.md`
- `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`

Regenerate these through their scripts when the underlying source or checklist state changes, then rerun the relevant contract tests.

## Artifact Readiness

Release artifact readiness still depends on:

- `docs/RELEASE_PROCESS.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `docs/RELEASE_BINARIES.md`
- `scripts/release_candidate_gate.sh --full`

Final release claims require tag-time artifact evidence. Local passing gates are necessary, but not sufficient, for a universal enterprise-readiness claim.
