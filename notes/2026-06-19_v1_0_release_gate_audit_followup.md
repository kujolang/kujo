# 2026-06-19 v1.0 Release Gate Audit Follow-Up

## Context

`bash scripts/release_candidate_gate.sh --full` reached optional `cargo audit` and failed on 2026-06-19 with four vulnerability advisories.

## Fixes Applied

- Updated the lockfile from `tokio-postgres 0.7.16` to `0.7.18`.
- Updated the lockfile from `postgres-protocol 0.6.10` to `0.6.12`.
- This resolves:
  - `RUSTSEC-2026-0178` (`tokio-postgres`)
  - `RUSTSEC-2026-0179` (`postgres-protocol`)
  - `RUSTSEC-2026-0180` (`postgres-protocol`)

## Explicit Audit Exception

`RUSTSEC-2023-0071` remains for `rsa 0.9.10`.

`cargo audit` reports `Solution: No fixed upgrade is available!` for this advisory. The dependency is present both directly and through `jsonwebtoken 10.3.0`, so the release gate now ignores only `RUSTSEC-2023-0071` explicitly:

```bash
cargo audit --ignore RUSTSEC-2023-0071
```

This exception does not ignore other vulnerability advisories. Existing unmaintained/yanked advisories still print as warnings under the current `cargo audit` default policy.

## Validation

- `cargo audit --ignore RUSTSEC-2023-0071` passed with warnings only after the lockfile updates.
