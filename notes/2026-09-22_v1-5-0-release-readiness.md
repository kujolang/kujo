# Kujo v1.5.0 release candidate

The AssetWorks follow-up explicitly requests a tested tagged runtime and installed-artifact verification. This compatible minor release packages the additive APIs accumulated since v1.4.0, including the confined streaming SHA-256/copy and bounded directory pagination required by AssetWorks. CHANGELOG.md records the scope. Language and CLI contract versions remain 1.0.0.

Work is isolated on codex/assetworks-confined-digest, PR #10. The original Kujo checkout and unrelated projects are untouched. Candidate metadata is 1.5.0; published-stable references remain v1.4.0 until publication succeeds. No Cargo registry release is requested. Existing GitHub/npm release automation remains the distribution mechanism.

Before publication: complete candidate native/capability/platform/release gates and the AssetWorks application matrix. After publication: verify downloaded checksummed artifacts and installation, then record exact run IDs, tags and checksums. This file is an acceptance checklist, not a claim that candidate or published-artifact verification has completed.

Local evidence before the metadata bump: 19 confined-filesystem unit tests and the copy primitive's VM/interpreter capability/parity integration test pass. Copy uses a fixed 64 KiB buffer, rejects source links/growth/oversize and destination replacement, preserves retained-parent publication semantics and requires both filesystem read and write capabilities. AssetWorks' full local 300-assertion application gate and 18 container boundary assertions pass against the API-equivalent development runtime.
