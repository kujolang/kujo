# AI-Native Enterprise Release Evidence

Status: active v1 release evidence index
Last updated: 2026-08-08

This index is the operator-facing evidence map for Kujo's stable v1 branch. It gives reviewers one place to find the commands, artifacts, and boundaries that support the current product posture.

## Readiness Boundary

Kujo `v1.0.0` ships the core AI-native mechanisms described in `docs/AI_RUNTIME.md`. The stable-release claim does not imply provider certification, a hosted control plane, or a universal enterprise-readiness warranty.

Release boundary: Kujo `v1.2.2` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

## Latest Local Verification Matrix

Run the full enterprise verification matrix before release or patch-tag review:

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

## Performance Evidence

AI-native pure-helper benchmarks are available as internal regression signals:

```bash
cargo bench --bench v1_perf_benchmarks -- ai_native_helpers --noplot --sample-size 10 --warm-up-time 0.5 --measurement-time 1
```

The group covers deterministic request hashing, schema validation, vector top-k scoring, and context fitting. Do not publish broad performance claims from this filtered run without preserving raw artifacts and environment details under the benchmark publication policy.

## Artifact Readiness

Release artifact verification is recorded through:

- `docs/RELEASE_PROCESS.md`
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`
- `docs/RELEASE_BINARIES.md`
- `scripts/release_candidate_gate.sh --full`

Stable release claims remain bounded by published artifacts and tag-time evidence. Local passing gates are necessary, but not sufficient, for a universal enterprise-readiness claim.
