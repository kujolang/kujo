# Kujo Field Notes — Published 1.8 installation defaults

**Date:** 2026-10-04
**Session:** 14:20 local
**Branch/Commit:** codex/v1-8-publication / release source 1acf4924fb96a8a660e86cdc35e4c4bf5933a304
**Scope:** Record verified v1.8 publication and promote stable runtime defaults

---

## What I Changed

Created and pushed the signed `v1.8.0` tag after the owner supplied the required
`UNBLOCK_V1_RELEASE` directive. The tag-time workflow published five native
archives, deterministic source, checksums, all five native npm packages, and the
neutral npm runtime package. Promoted repository installer, setup action,
release documentation, and stable-version contracts to 1.8.0 only after both
published-install matrices passed.

## Gotchas (Read This Next Time)

The npm registry briefly returned per-package 404 responses while processing the
successful trusted publications. Wait for all six exact package versions to be
visible before dispatching the five-target npm clean-install matrix. A successful
tag workflow alone is not published-install evidence.

## Things I Learned

The deterministic source archive introduced during candidate preparation passed
its published checksum and embedded-version validation alongside all five native
targets. Crates.io remains optional and was not published.

## Debug Notes (Only if applicable)

- Signed tag: `v1.8.0` -> `1acf4924fb96a8a660e86cdc35e4c4bf5933a304`.
- Local SSH tag verification: good signature for `deviodigital@gmail.com`, key
  fingerprint `SHA256:jXbytvrzJb3vhExofE7w1VilK9pY6MrJkXDmXlHycGA`.
- GitHub tag verification: `verified: true`, reason `valid`.
- Release workflow `37219738234`: passed all ten jobs, including five native
  builds, source packaging, GitHub release publication, and six npm packages.
- Published-artifact smoke `37223624965`: passed native/source install,
  checksum, metadata, and version validation.
- npm clean-install smoke `37223833441`: passed on Linux x64/arm64, macOS
  x64/arm64, and Windows x64.
- All six exact `@kujolang/*@1.8.0` packages became visible in the npm registry.

## Follow-ups / TODO (For Future Agents)

Keep v1.8.0 stable defaults synchronized between this repository,
`kujolang/kujolang.ai`, and `kujolang/docs.kujolang.ai`. Begin any later release
from a new versioned development change rather than rewriting the signed tag.

## Links / References

- [GitHub release](https://github.com/kujolang/kujo/releases/tag/v1.8.0)
- [Publication workflow](https://github.com/kujolang/kujo/actions/runs/37219738234)
- [Published artifact smoke](https://github.com/kujolang/kujo/actions/runs/37223624965)
- [npm install smoke](https://github.com/kujolang/kujo/actions/runs/37223833441)
- [Publication record](../release/kujo-1.8.0-publication.json)
- [Release notes](../docs/RELEASE_NOTES_1_8.md)
