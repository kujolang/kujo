# Contract Version Freeze And CI Drift Guard (v1.0.0 P0)

Date: 2026-05-01

## Summary

Completed the next highest-priority open v1.0.0 P0 roadmap item:

- froze and aligned CLI/LSP/language contract metadata to a unified v1 draft baseline
- added a CI guard to fail on future contract-doc/version drift

## Implementation

### Contract metadata alignment

Updated baseline markers to a single contract status/version in:

- `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `Status: v1.0.0 baseline draft (active)`
  - `Contract version: 1.0.0-draft`
- `docs/PROTOCOL_CONTRACTS.md`
  - `Status: v1.0.0 baseline draft (active)`
  - `Contract version: 1.0.0-draft`
- `docs/LANGUAGE_SPEC.md`
  - `Status: v1.0.0 baseline draft (active)`
  - `Spec version: 1.0.0-draft`

### CI drift guard

Added `.github/scripts/check-contract-version-sync.sh` to enforce metadata consistency across the three docs.

Wired script into `.github/workflows/release-state-guard.yml` so CI fails when contract markers diverge.

## Verification Evidence

Commands executed:

1. `bash .github/scripts/check-release-state.sh`
- Result: PASS

2. `bash .github/scripts/check-contract-version-sync.sh`
- Result: PASS

3. `cargo test --test cli_json_contracts`
- Result: PASS
- Passed: 4
- Failed: 0

## Roadmap Impact

Marked complete in `ROADMAP.md`:

- `P0`: Freeze and verify CLI/LSP contract versioning for v1 baseline

## Next Highest-Priority Open v1.0.0 Item

- `P0`: Expand negative-path contract fixtures for automation reliability.
