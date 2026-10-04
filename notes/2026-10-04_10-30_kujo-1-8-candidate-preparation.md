# Kujo Field Notes — 1.8 candidate preparation

**Date:** 2026-10-04
**Session:** 10:30 local
**Branch/Commit:** codex/v1-8-final-prep / local rehearsal at 59ca3e106baad2373cc336bf7cfa721474e165dd
**Scope:** Final unpublished v1.8 release packaging, evidence, and documentation preparation

---

## What I Changed

Started from clean, synchronized `main` at
`06923afa141995fb6d4f2940a820515583676e16`. Restored the canonical `/var/`
artifact-ignore entry that had left the post-merge tool-artifact guard red.
Generalized the stale 1.6-only local RC rehearsal to the current semantic
version, added the candidate-declared deterministic source archive to the tagged
release workflow, and added published checksum/version validation. Added release
packaging contract tests and aligned the roadmap, changelog, release process,
candidate notes, and readiness record. Corrected the roadmap's stale v1.7 source
identity to `813072040a1ac643312f5163fcfa4f26474c9095`.

## Gotchas (Read This Next Time)

Candidate metadata at 1.8.0 does not make 1.8 public. Keep stable installer,
README, setup-action, docs-site, and npm guidance on 1.7.0 until the signed tag
and all hosted artifacts pass published-install checks. Run checksum validation
from the artifact directory because each checksum intentionally records only the
archive basename. The release policy still requires the literal
`UNBLOCK_V1_RELEASE` directive before tagging or publication.

## Things I Learned

The candidate metadata already promised `kujo-v1.8.0-source.tar.gz`, but the
tagged workflow did not produce it and the published smoke did not validate it.
The runtime's generic ShipCheck scan passes its blocking gate but reports four
known detector warnings: Cargo lint, Cargo format, intentional absence of
`kennel.toml`, and the Cargo binary entry point.

## Debug Notes (Only if applicable)

- Local RC at `59ca3e106baad2373cc336bf7cfa721474e165dd` used Rust/Cargo 1.96.0 on macOS x64.
- Binary SHA-256: `99ff38f3d8c97f9fc89da76f72273c4eef14a01d41dcfbfc2eaccce327f82856`.
- Native archive SHA-256: `bf4bbfcb71a053ae29fa2b2df457ffc68bbfdd409219cb7dcbf46644e178aa91`.
- Source archive SHA-256: `86bdaaf83596eb9212279a0e8e508b6cb1dcdc9daa4c9793fa68c0fc00b7ddab`.
- `bash scripts/release_candidate_gate.sh --full`: passed; 977 library tests
  passed, 7 ignored, all integration suites passed, 123 parity cases passed,
  both fixture modes passed 150/150 with 11 declared skips, and the 31-case
  serve suite passed.
- `cargo audit --deny warnings --ignore RUSTSEC-2025-0141`: passed;
  `cargo-deny` was unavailable locally and remains covered by hosted CI.
- `bash scripts/fuzz_smoke.sh --max-total-time 10`: all six lexer, parser,
  bounded XML, bounded gzip, bounded single-entry ZIP, and PDF HTML-profile
  targets completed without a crash.
- `npm test --prefix npm`: 12 passed; `npm run pack:dry-run --prefix npm`:
  all six version-aligned 1.8.0 packages packed successfully.
- VS Code extension `npm ci && npm run check`: passed with zero npm audit findings.
- ShipCheck `scan` and `gate`: exit 0, `gate_passed=1`, 12 passed, 4 documented
  warnings, 0 error failures.

## Follow-ups / TODO (For Future Agents)

Do not tag or publish until the exact release directive is present. After it is,
create the signed `v1.8.0` tag from verified `main`; monitor the five native
builds, source archive, checksums, all six npm packages, and published smoke;
then record publication evidence and only afterward promote repository and
website stable defaults. Crates.io is not required for this release.

## Links / References

- [1.8 release notes](../docs/RELEASE_NOTES_1_8.md)
- [1.8 readiness record](../docs/KUJO_1_8_RELEASE_READINESS.md)
- [Release policy](../docs/RELEASE_PROCESS.md)
- [1.8 candidate metadata](../release/kujo-1.8.0-rc.json)
- [Local RC receipt](../docs/evidence/kujo-1.8-rc/local-rc-macos-x64.json)
- [Original post-merge artifact-guard failure](https://github.com/kujolang/kujo/actions/runs/37194219931)
