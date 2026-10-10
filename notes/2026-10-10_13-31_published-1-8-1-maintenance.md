# Kujo Field Notes — Published 1.8.1 maintenance release and 1.9 roadmap

**Date:** 2026-10-10
**Session:** 13:31 local
**Branch/Commit:** main / signed release source 357796a1a868f2c80bc8f7b8edcc7d6aff384f04
**Scope:** Publish the maintenance baseline, verify distribution, and prepare post-release roadmap items 2–6

---

## What I Changed

Published signed `v1.8.1` after the owner explicitly requested the maintenance
release and post-release roadmap. All five native platforms, deterministic
source/checksums and six npm packages are published. Fresh native/npm installs
and native upgrades from 1.8.0 pass across all five platforms. Stable installer,
setup action, documentation and version contracts now point to 1.8.1.

Roadmap item 1 is complete for core runtime distribution. Item 2 is ready:
native value representation and VM/interpreter parity. Items 3–6 define measured
release-build performance, token/payload/output efficiency, a conditional
`yield from` design, and human first-hour validation. Owners, dependencies and
completion criteria are explicit. No new language feature was implemented in
this release-preparation task.

## Gotchas (Read This Next Time)

The initial native smoke `38071577029` and upgrade smoke `38071623303`
reported GitHub denial/rate limiting on macOS ARM64 after verifying the archive
and successfully running the new binary. Other targets passed. The unchanged
checks subsequently passed in `38071979374`, including actual 1.8.0 upgrades.
Commit `34d716f10c9671439dcf1fec51c4af0f97498c0e` adds failure-only HTTP header
diagnostics for future failures; it does not suppress failures, retry tests,
change test timeouts, or alter the signed release artifacts.

npm publication success preceded registry visibility. Early exact-version reads
returned 404, and the first macOS ARM64 npm smoke lacked its optional platform
package. After all six versions resolved with integrity metadata and no install
lifecycle hooks, the failed clean-install job passed on attempt 2. Initial
failures remain in workflow history and local evidence; none was waived.

The candidate identity manifest was initially missing after the version bump.
It was added and the full gate rerun successfully before tagging. See the
candidate note for that correction and the baseline commands.

## Things I Learned

A version bump, signed tag, native publication, npm publication and installed
runtime verification are distinct evidence milestones. Promote public defaults
only after both installation channels pass. Core publication does not prove
that separate websites or deployed catalogs have been updated.

## Debug Notes (Only if applicable)

- Release source: `357796a1a868f2c80bc8f7b8edcc7d6aff384f04`.
- SSH tag signature verified locally and by GitHub (`verified: true`, `valid`).
- Full source CI: `38067375815`; all main-branch workflows for the signed source
  passed, including Windows process/GIF conformance and all native upgrade jobs.
- Publication workflow `38067801088`: all ten jobs passed; 13 uploaded release
  assets and six exact npm package versions were independently read back.
- Native/source installation and 1.8.0 upgrade matrix `38071979374`: all six jobs
  passed (five platforms plus source archive verification).
- npm installation matrix `38071766773`, attempt 2: all five platforms passed.
- Full local gate: 3,017 Rust tests passed, 0 failed, 18 ignored; dual and
  interpreter fixture runs each passed 150 with 11 explicit skips. Formatting,
  Clippy, focused security/package/parity/socket tests and dependency audit passed.
- `cargo test --test serve_command_integration -- --test-threads=1` also passed.
- `npm test --prefix npm`: 12 passed. `npm run pack:dry-run --prefix npm`: all six
  packages passed. Optional cargo-deny was unavailable; no release performance
  claim is inferred from this metadata/docs task.
- Post-publication `cargo fmt --check`, `bash .github/scripts/check-release-state.sh`,
  `KUJO_RELEASE_TAG=v1.8.1 bash .github/scripts/check-tag-version.sh`,
  `bash tests/install_release_manifest.sh`, and
  `bash scripts/release_candidate_gate.sh --roadmap-only` all passed.
- The following post-publication contract command passed with the same Cargo
  profile environment used for the full local gate:

  ```bash
  cargo test --test readme_contracts --test architecture_docs_contract \
    --test docs_policy_consistency_contract \
    --test v1_maturity_boundary_alignment_contract --test release_candidate_identity \
    --test release_process_docs_contract --test release_packaging_contract \
    --test release_candidate_gate_contract
  ```

  Local verbose evidence remains in `.audit-evidence/release-1.8.1/`.

## Follow-ups / TODO (For Future Agents)

Begin roadmap item 2 from the verified 1.8.1 baseline. Do not rewrite its tag.
External promotion remains open: website maintainers own installer/site updates,
MCP catalog maintainers own catalog refresh/deployment, and course site
maintainers own course revalidation/deployment. The current repository write
scope prevented those external changes; the publication receipt records each
repository, owner and required verification. No sibling repository was modified.
Crates.io remains optional and was not published.

## Links / References

- [Release](https://github.com/kujolang/kujo/releases/tag/v1.8.1)
- [Publication workflow](https://github.com/kujolang/kujo/actions/runs/38067801088)
- [Native/source and upgrade smoke](https://github.com/kujolang/kujo/actions/runs/38071979374)
- [npm smoke](https://github.com/kujolang/kujo/actions/runs/38071766773)
- [Publication receipt](../release/kujo-1.8.1-publication.json)
- [Candidate evidence](2026-10-10_12-15_kujo-1-8-1-candidate.md)
- [Maintenance notes](../docs/RELEASE_NOTES_1_8_1.md)
- [Roadmap](../ROADMAP.md)
