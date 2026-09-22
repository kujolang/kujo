# Kujo Field Notes — v1.5.0 release readiness

**Date:** 2026-09-22
**Session:** 16:37 local
**Branch/Commit:** codex/assetworks-confined-digest / a0d433e
**Scope:** AssetWorks supporting runtime and compatible Kujo 1.5.0 release

---

## What I Changed

Prepared the 1.5.0 crate/npm metadata and changelog. Added confined streaming copy with atomic no-replace publication. Merged upstream bounded stdin/digest and lexical-scope fixes without modifying the user's original Kujo checkout. The AssetWorks follow-up explicitly requests a tested tagged runtime and installed-artifact verification.

Aligned the source installer's default and help example with the 1.5.0 candidate, and verified its release-manifest/default/override contract. Public stable-release prose remains at the published version until artifact verification succeeds.

## Gotchas (Read This Next Time)

Published-stable references stayed at v1.4.0 during preparation and moved to v1.5.0 only after published-artifact verification. No Cargo registry release is requested. Existing GitHub/npm automation remains the distribution mechanism. Regenerate source inventories after Rust line changes. New notes must follow this repository's complete field-note template.

## Things I Learned

Copy requires both filesystem capabilities and returns the digest of the bytes actually copied; it does not promise a source snapshot. The fixed buffer is 64 KiB and the caller ceiling is at most 4 GiB. AssetWorks can preserve signatures through disaster recovery by retaining record bytes and resolving restored artifacts by digest.

## Debug Notes (Only if applicable)

The merged documentation initially duplicated the `list_dir_beneath` inventory row. Hosted release gate 35781463163 caught this after the runtime suites passed; removed the duplicate and retained the more detailed canonical row. This was documentation drift, not a waived test.

The artifact gate 35784868167 then caught the new copy API missing from the separate tiered standard-library reference. Added its bounded copy, publication, receipt and capability contract there and ran the related documentation policy suites together before retrying publication builds.

At a0d433e, 21 confined filesystem unit tests, five VM/interpreter boundary integration tests and upstream bounded stdin/rooted digest tests pass locally. The earlier full local application gate passed 300 assertions; isolation passed 21 after explicit no-fallback error handling. Hosted release verification is pending. The local inventory test's VM fixture subtest expects target/debug/kujo and cannot use the separate CARGO_TARGET_DIR automatically; the two source-inventory freshness tests passed.

## Follow-ups / TODO (For Future Agents)

Native publication and downloaded-install verification are complete, with receipts below. The independent npm registry authorization remains an open maintainer follow-up. Language/CLI contract versions remain 1.0.0.

## Links / References

- Runtime PR: https://github.com/kujolang/kujo/pull/10
- AssetWorks acceptance: https://github.com/kujolang/assetworks/blob/main/docs/NEXT_SESSION_IMPLEMENTATION.md
- Release mechanics: .github/workflows/release-binaries.yml and .github/workflows/release-published-artifact-smoke.yml

## Published result

Signed `v1.5.0` targets `cc2d7dbb59a8dc05f00d629e100932f56f4062f6`, integrated by merge `44aaf58a7fd1fbcecff46bdaddac614f99e052d7` with an identical tree. Run 35787043614 passed the complete gate, hardened RAG integration, all five native builds and npm packaging. Native/npm binary digests and full source metadata were checked before publication. All five native archive SHA-256 values are bound into the SSH-signed tag, verified by GitHub.

Published-download run 35793560616 passed on all five platforms; its first macOS ARM64 attempt hit a GitHub public API rate limit after successful download/execution, and the unchanged failed-job retry passed. The tag-triggered duplicate build 35793320106 was cancelled because the already-verified exact-revision artifacts from 35787043614 were published, preserving the signed archive digests.

The independent npm publisher run 35793388734 failed with registry E404 on the existing @kujolang/kujo-darwin-arm64 package; public lookup still reports 1.4.0. npm 1.5.0 publication is not claimed. Native GitHub archives remain canonical, and no Cargo registry publication was attempted. Registry authorization requires maintainer review; no trust or credential boundary was bypassed.

AssetWorks v0.3.0 is signed and published from 1cef9b0. Its final installation/website receipts and next-session scope are maintained in AssetWorks docs/NEXT_SESSION_IMPLEMENTATION.md. The public installer synchronization is isolated in kujolang.ai commit 8d60dd0.

Installation guidance distinguishes verified native 1.5.0 from the still-published npm 1.4.0 package; npm lookup confirmed both the resolver and darwin-arm64 package remain 1.4.0. Standard-library availability labels now identify the APIs shipped in 1.5.0.
