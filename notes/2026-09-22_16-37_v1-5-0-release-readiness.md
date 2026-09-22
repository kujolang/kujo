# Kujo Field Notes — v1.5.0 release readiness

**Date:** 2026-09-22
**Session:** 16:37 local
**Branch/Commit:** codex/assetworks-confined-digest / a0d433e
**Scope:** AssetWorks supporting runtime and compatible Kujo 1.5.0 release

---

## What I Changed

Prepared the 1.5.0 crate/npm metadata and changelog. Added confined streaming copy with atomic no-replace publication. Merged upstream bounded stdin/digest and lexical-scope fixes without modifying the user's original Kujo checkout. The AssetWorks follow-up explicitly requests a tested tagged runtime and installed-artifact verification.

## Gotchas (Read This Next Time)

Published-stable references remain v1.4.0 until publication succeeds. No Cargo registry release is requested. Existing GitHub/npm automation remains the distribution mechanism. Regenerate source inventories after Rust line changes. New notes must follow this repository's complete field-note template.

## Things I Learned

Copy requires both filesystem capabilities and returns the digest of the bytes actually copied; it does not promise a source snapshot. The fixed buffer is 64 KiB and the caller ceiling is at most 4 GiB. AssetWorks can preserve signatures through disaster recovery by retaining record bytes and resolving restored artifacts by digest.

## Debug Notes (Only if applicable)

At a0d433e, 21 confined filesystem unit tests, five VM/interpreter boundary integration tests and upstream bounded stdin/rooted digest tests pass locally. The earlier full local application gate passed 300 assertions; isolation passed 21 after explicit no-fallback error handling. Hosted release verification is pending. The local inventory test's VM fixture subtest expects target/debug/kujo and cannot use the separate CARGO_TARGET_DIR automatically; the two source-inventory freshness tests passed.

## Follow-ups / TODO (For Future Agents)

Complete candidate platform/release gates and AssetWorks' matrix before publication. Verify published checksummed artifacts and installation afterwards, then record exact tags, run IDs and checksums. This is an acceptance checklist, not a completed publication claim. Language/CLI contract versions remain 1.0.0.

## Links / References

- Runtime PR: https://github.com/kujolang/kujo/pull/10
- AssetWorks acceptance: https://github.com/kujolang/assetworks/blob/main/docs/NEXT_SESSION_IMPLEMENTATION.md
- Release mechanics: .github/workflows/release-binaries.yml and .github/workflows/release-published-artifact-smoke.yml
