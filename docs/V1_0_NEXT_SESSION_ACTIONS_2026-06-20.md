# Kujo v1.0 Next Session Actions (2026-06-20)

Status: current handoff after final review hardening and dependency advisory
refresh passes on 2026-06-20

## Release-Flight Blockers

1. Complete `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` after release owners
   provide `UNBLOCK_V1_RELEASE`.
2. Record release URLs, per-asset SHA-256 values, published-artifact smoke
   result, and command logs in dated `notes/` evidence.

## External Deferrals

1. `cargo-deny` is still not installed in this local toolchain. The release
   gate keeps `cargo deny check` as an optional command and keeps `cargo audit`
   as the active fallback until the tool is installed and a policy file can be
   added intentionally.
2. Remaining `cargo audit --ignore RUSTSEC-2023-0071` warnings are scoped to
   optional/transitive upstream surfaces with no safe local replacement in this
   readiness pass:
   - `RUSTSEC-2020-0168` (`mach`) through experimental optional JIT support.
   - `RUSTSEC-2024-0436` (`paste`) through optional image lockfile metadata.
   - `RUSTSEC-2023-0071` (`rsa`) remains explicitly ignored by the release
     gate because the advisory currently has no fixed upgrade.

## Verification Starting Point

Use the release-candidate gate first, then add focused tests for the touched
area:

```bash
KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_candidate_gate.sh --full
```
