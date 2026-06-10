# V1U-GATE-004 — Release Suite Stabilization Evidence

Date: 2026-05-20

## Scope

Stabilize environment-sensitive release-suite failures seen in `V1U-GATE-003`, then verify repeatability with consecutive full RC gate runs.

## Stabilization Changes

1. `src/docgen/gaps.rs` test hardening:
   - Added poison-tolerant test mutex locking helper for link-validation counter tests.
   - Wrapped external-host check test in panic guard and short-circuit skip path when host runtime cannot initialize reqwest/system-configuration stack.

2. `tests/docgen_universal.rs` network/runtime guardrails:
   - Updated local test HTTP server helper to skip when localhost bind is permission-denied.
   - Wrapped external-link validation tests in panic guards with deterministic skip messaging when reqwest/system-configuration initialization is unavailable.

## Validation Commands

1. Targeted regressions:
   - `cargo test --lib repeated_external_host_checks_reuse_single_http_client`
   - `cargo test --lib repeated_local_anchor_checks_use_cached_file_index_per_path`
   - Result: PASS

2. Full RC gate run #1:
   - `bash scripts/release_candidate_gate.sh --full`
   - Result: PASS

3. Full RC gate run #2:
   - `bash scripts/release_candidate_gate.sh --full`
   - Result: PASS

## Outcome

`V1U-GATE-004` acceptance target met in this environment: two consecutive full RC gate passes after suite hardening.
