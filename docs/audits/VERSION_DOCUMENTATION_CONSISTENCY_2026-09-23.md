# Kujo Version and Documentation Consistency Audit

Audit date: 2026-09-23
Repository: `kujolang/kujo`
Audited development revision: `b811f24` plus the corrective changes listed below
Stable release audited: `v1.5.0` (`cc2d7db`, annotated tag object `83fbdd4`)

## A. Executive Summary

The authoritative current Kujo release is **1.5.0**. `Cargo.toml`, the CLI,
the installer default, the latest Git tag, the latest GitHub Release, the five
native archives, and the release checksums agree. The tag and release commit
also resolve to the same tree, and GitHub reports the tag signature as verified.

The audit found 12 actionable repository issues: five P1, six P2, and one P3.
All 12 are corrected by the accompanying commits. It also confirmed one open
external distribution mismatch: source npm manifests are 1.5.0, while the
public npm packages remain at 1.4.0 because the 1.5.0 publish attempt was not
authorized. Current documentation now states that limitation. No ambiguous
version was found.

Four important version families are intentionally independent of the Kujo
release: the stable language/CLI/protocol contract remains 1.0.0; installer,
editor-extension, and Tree-sitter package versions remain 0.1.0; fixture and
example package versions are local test data; and historical release records
retain the versions of the releases they describe.

Overall consistency after remediation is good. Remaining release risk is
concentrated in external publishing: GitHub binaries are current, npm is one
minor release behind, and no `kujolang` crate is published on crates.io.

Counts (pre-fix repository state):

- Confirmed actionable repository issues: 12
- Stale-documentation findings within those issues: 9
- Current external version mismatches: 1
- Intentional version families: 4
- Ambiguous findings: 0
- Broken executable Kujo examples: 0

## B. Previously Flagged Claims — Verdict

| Previously flagged claim | Verdict | Evidence |
| --- | --- | --- |
| Kujo current release is 1.5.x | ✅ Confirmed | Latest local/remote tag and GitHub Release are `v1.5.0`; Cargo and CLI report `1.5.0`. |
| Some `1.0.0` references are stale | ⚠️ Partially true | Most are intentional contract, schema, fixture, or historical values. The active release-binary command examples were stale and are corrected. |
| npm/package is still 1.4.0 | ✅ Confirmed | The public runtime and five platform packages resolve to 1.4.0; repository manifests are 1.5.0. Release notes record the failed authorization attempt. |
| README has old version references | ❌ False | The main README already identified 1.5.0 correctly. Its 1.0, 1.3, and 1.4 references describe compatibility/history rather than the current release. Only its Rust MSRV statement was stale. |
| INSTALLATION docs contain old version references | ⚠️ Partially true | The main guide correctly distinguished native 1.5.0 from npm 1.4.0, but the Rust MSRV was too low and related installation documents contained stale examples. |
| ROADMAP contains stale information | ✅ Confirmed | It still called the 1.4 line current and said documentation was aligned on 1.4.0. |

## C. Complete Findings Table

| Severity | Classification | File | Line/area | Issue | Recommended fix | Status |
| --- | --- | --- | --- | --- | --- | --- |
| P1 | CONFIRMED BUG | `README.md`, `INSTALLATION.md`, `docs/RELEASE_ARTIFACT_VALIDATION.md` | source-build prerequisites | Claimed Rust 1.86, but the locked dependency graph cannot compile on 1.86; `aes 0.9.3` requires 1.89. | Declare Rust 1.89 in Cargo/docs and test the lockfile with 1.89 in CI. | Resolved |
| P1 | CONFIRMED BUG | `src/upgrade.rs`, `docs/RUNTIME_UPGRADE.md` | Cargo-managed upgrade guidance | Recommended `cargo install kujolang`, but no `kujolang` crate is published. | Direct Cargo-owned installs to their source checkout or a GitHub archive. | Resolved |
| P1 | STALE DOCUMENTATION | `.github/actions/setup-kujo/action.yml`, `docs/SETUP_KUJO_ACTION.md` | default/example version | The setup action default and its public example installed 1.2.3. | Set both to 1.5.0 and enforce them in the release-state check. | Resolved |
| P1 | CONFIRMED BUG | `docs/ECOSYSTEM_INSTALL.md` | one-command install and Dispatch example | The command forced `--ref v1.2.3` across ecosystem repositories that do not share that tag; the Dispatch v1.1.0 manifest URL returned 404. | Use the stable installer default and Dispatch's published v1.2.0 manifest. | Resolved |
| P1 | STALE DOCUMENTATION | `docs/BUILD_AN_AGENT.md` | installation heading | Called 1.2.3 the stable runtime. | State 1.5.0 and enforce it in the release-state check. | Resolved |
| P2 | VERSION MISMATCH | `npm/**/package.json` versus npm registry | all six packages | Repository npm packages are 1.5.0, but registry `latest` is 1.4.0. | Restore npm publisher authorization and publish the already-generated 1.5.0 packages; keep native archives canonical until verified. | Open, external |
| P2 | VERSION MISMATCH | `src/repl.rs` | REPL banner | The CLI reported 1.5.0 while the REPL hard-coded 0.5.0. | Render the banner from `CARGO_PKG_VERSION`. | Resolved |
| P2 | STALE DOCUMENTATION | `ROADMAP.md` | opening/current-state text | Called 1.4 current and described docs as aligned to 1.4. | Describe the 1.5 line and explicitly record the npm lag. | Resolved |
| P2 | STALE DOCUMENTATION | `docs/RELEASE_BINARIES.md` | manual POSIX/PowerShell examples | Active download examples pinned v1.0.0. | Use v1.5.0 examples; retain historical v1.0 checklists unchanged. | Resolved |
| P2 | STALE DOCUMENTATION | `docs/CONCURRENCY.md`, `docs/MEMORY.md`, `docs/EXTENDING.md`, `docs/VM_INSTRUCTIONS.md` | document headers/current claims | v0.9 implementation notes looked like current release contracts, including obsolete synchronous-async statements. | Mark them historical/implementation references and point to current canonical sources. | Resolved |
| P2 | STALE DOCUMENTATION | `.github/AGENT_INSTRUCTIONS.md`, `.github/HOW_TO_START_AGENT_SESSION.md`, `.github/IMPLEMENTATION_GUIDE.md`, active release prompt | document headers/current cycle | Pre-1.0 architecture and v0.12 cycle text appeared operationally current. | Add historical warnings and make the active prompt defer to current `AGENTS.md` and `ROADMAP.md`. | Resolved |
| P3 | STALE DOCUMENTATION | `docs/kujo-language-helper-audit-2026-07-10/README.md` | report links | Linked three report files that do not exist. | Link the retained machine-readable candidate ledger instead. | Resolved |
| P2 | FUTURE MAINTENANCE RISK | `.github/scripts/check-release-state.sh`, release CI | release synchronization | Existing checks covered only a subset of duplicated versions and did not test the declared MSRV. | Check installer/action/docs/npm source versions and add a locked MSRV CI job. | Mitigated |
| P3 | NOT AN ISSUE | `Cargo.toml` on `main` versus `v1.5.0` tag | development state | `main` contains post-tag changes while remaining version 1.5.0. | Keep those changes under `[Unreleased]`; bump only as part of the next release. | No change |

## Detailed Findings

### Finding: Source-build MSRV was impossible

Classification: CONFIRMED BUG
Severity: P1
Status: Confirmed and fixed

Location: `Cargo.toml:1-6`, `README.md` and `INSTALLATION.md` source prerequisites

Current value before fix: Rust 1.86 or newer; no Cargo `rust-version`.

Expected/intended value: Rust 1.89 or newer, declared in package metadata and
validated against the locked dependency graph.

Evidence: `cargo +1.86.0 check --locked` fails before compiling Kujo because
locked dependencies require newer compilers; the highest requirement reported
is `aes 0.9.3` at Rust 1.89. The same command under Rust 1.89 succeeds. The
default toolchain succeeding did not validate the documented minimum.

Recommended fix: add `rust-version = "1.89"`, align prerequisite prose, and run
`cargo check --locked` with 1.89 in release CI.

### Finding: Cargo upgrade guidance named an unpublished crate

Classification: CONFIRMED BUG
Severity: P1
Status: Confirmed and fixed

Location: `src/upgrade.rs` (`guidance`), `docs/RUNTIME_UPGRADE.md`

Current value before fix: `cargo install kujolang --version VERSION`.

Expected/intended value: source-checkout reinstall or official GitHub archive.

Evidence: the crates.io sparse-index path for `kujolang` returns 404 and
`cargo search '^kujolang$'` returns no package. The v1.5.0 GitHub Release notes
also state that no Cargo registry publication was performed.

Recommended fix: do not advertise a registry command until publication is
verified; test the emitted guidance.

### Finding: Public npm is behind repository metadata

Classification: VERSION MISMATCH
Severity: P2
Status: Confirmed; unresolved external publication

Location: `npm/package.json`, `npm/runtime/package.json`,
`npm/platforms/*/package.json`; public npm registry

Current value: source packages 1.5.0; public packages 1.4.0.

Expected/intended value: npm 1.5.0 when publisher access permits, or explicit
documentation that 1.4.0 is a separate, older distribution channel.

Evidence: registry metadata for `@kujolang/kujo-runtime` and each of its five
platform packages returns `latest: 1.4.0`. A clean temporary installation runs
`kujo 1.4.0`. The release workflow successfully generated 1.5.0 npm tarballs,
but the v1.5.0 release notes record E404/authorization failure during publish.

Recommended fix: repair npm org/package authorization, publish all six 1.5.0
packages atomically, verify a clean install on supported targets, then remove
the temporary 1.4.0 warning from current install docs.

### Finding: Setup action silently selected an older runtime

Classification: STALE DOCUMENTATION
Severity: P1
Status: Confirmed and fixed

Location: `.github/actions/setup-kujo/action.yml:4-8`,
`docs/SETUP_KUJO_ACTION.md`

Current value before fix: default and example `v1.2.3`.

Expected/intended value: current stable native release `v1.5.0`.

Evidence: the action downloads the exact release named by this input, and the
v1.5.0 release provides assets for every action-supported target. No comment or
compatibility requirement justified retaining 1.2.3.

Recommended fix: update the default/example and add both to the release-state
guard.

### Finding: Ecosystem install examples were not executable as written

Classification: CONFIRMED BUG
Severity: P1
Status: Confirmed and fixed

Location: `docs/ECOSYSTEM_INSTALL.md`

Current value before fix: force `--ref v1.2.3` for a multi-repository install;
fetch `dispatch-v1.1.0.refs` from a path that returns 404.

Expected/intended value: use Kujo's current installer default and a product's
own reviewed release manifest.

Evidence: the ecosystem repositories referenced by the installer do not share
a common `v1.2.3` tag. Dispatch's current release manifest exists at
`v1.2.0/release/dispatch-v1.2.0.refs`; the documented v1.1.0 URL does not.

Recommended fix: remove the shared-ref assumption and use the verified Dispatch
manifest URL.

### Finding: REPL advertised 0.5.0

Classification: VERSION MISMATCH
Severity: P2
Status: Confirmed and fixed

Location: `src/repl.rs` banner rendering

Current value before fix: `Kujo REPL v0.5.0`; CLI `kujo 1.5.0`.

Expected/intended value: the REPL bundled with the CLI reports the package
version unless a separately versioned REPL is formally introduced.

Evidence: the banner value originated in the initial implementation and had no
independent version source, release process, or compatibility document. The
REPL ships inside the same `kujolang` binary.

Recommended fix: use `env!("CARGO_PKG_VERSION")` and test the banner.

### Finding: Agent onboarding named 1.2.3 as stable

Classification: STALE DOCUMENTATION
Severity: P1
Status: Confirmed and fixed

Location: `docs/BUILD_AN_AGENT.md:7`

Current value before fix: “Install the stable Kujo v1.2.3 runtime”.

Expected/intended value: current native stable release v1.5.0.

Evidence: Cargo, CLI, installer, tag, GitHub Release, and native archives all
identify v1.5.0. The guide supplied no compatibility reason to pin 1.2.3 and
called it “stable,” so a new user would receive an obsolete release description.

Recommended fix: name v1.5.0 and cover the statement in the release-state gate.

### Finding: Roadmap still described 1.4 as current

Classification: STALE DOCUMENTATION
Severity: P2
Status: Confirmed and fixed

Location: `ROADMAP.md` opening, current state, and package-trust sections

Current value before fix: “current 1.4 line,” docs aligned on 1.4.0.

Expected/intended value: 1.5 is current; npm remains 1.4.0.

Evidence: the same file linked stable `v1.5.0`, while GitHub/Cargo/CLI agreed on
1.5.0. This was internal contradiction, not independent component versioning.

Recommended fix: update current-state prose while retaining accurate 1.4
historical attribution.

### Finding: Active binary examples downloaded 1.0.0

Classification: STALE DOCUMENTATION
Severity: P2
Status: Confirmed and fixed

Location: `docs/RELEASE_BINARIES.md` manual POSIX and PowerShell examples

Current value before fix: `KUJO_VERSION=v1.0.0`.

Expected/intended value: current example `v1.5.0`; historical release
checklists must remain unchanged.

Evidence: GitHub v1.5.0 publishes all filenames used by the documented naming
algorithm, and its macOS x64 archive was checksum-verified and executed during
this audit.

Recommended fix: update only the active manual examples.

### Finding: Historical implementation documents looked current

Classification: STALE DOCUMENTATION
Severity: P2
Status: Confirmed and fixed

Location: `docs/CONCURRENCY.md`, `docs/MEMORY.md`, `docs/EXTENDING.md`,
`docs/VM_INSTRUCTIONS.md`, and legacy `.github` agent documents

Current value before fix: v0.9/v0.12/pre-1.0 implementation claims without a
prominent historical boundary.

Expected/intended value: retain useful history without allowing it to override
the canonical v1.5 architecture, language, and runtime documentation.

Evidence: current source has a Tokio-backed async runtime and a VM-default
execution pipeline, contradicting old synchronous-async and tree-walker-first
descriptions. Git history shows these documents predate the current release.

Recommended fix: add explicit status banners and canonical links rather than
rewriting historical evidence.

### Finding: Helper-audit index linked deleted reports

Classification: STALE DOCUMENTATION
Severity: P3
Status: Confirmed and fixed

Location: `docs/kujo-language-helper-audit-2026-07-10/README.md:7-13`

Current value before fix: three Markdown links to report artifacts absent from
the repository.

Expected/intended value: the index links only retained evidence.

Evidence: repository-relative resolution failed for each report target, while
`helper-candidates.json` exists and contains the retained machine-readable
candidate ledger.

Recommended fix: replace the dead report list with a link to the retained
ledger; do not fabricate missing historical reports.

### Finding: Version synchronization was only partially automated

Classification: FUTURE MAINTENANCE RISK
Severity: P2
Status: Confirmed and mitigated

Location: `.github/scripts/check-release-state.sh`,
`.github/workflows/ci-release-gate.yml`

Current value before fix: Cargo/README/Roadmap/tag checks existed, but installer,
setup action, active examples, npm source manifests, and MSRV prose were not all
linked to Cargo metadata.

Expected/intended value: one release-state gate detects drift before tagging,
while independent contract/package versions remain explicitly excluded.

Evidence: every stale current-release value found by this audit was outside the
old guard. The release workflow builds npm tarballs but registry publication
still depends on external authorization.

Recommended fix: extend the local guard, add MSRV CI, and add a post-publish npm
registry verification job that cannot pass on tarball generation alone.

## D. Version Matrix

| Component | Observed version | Source of truth | Should match Kujo release? |
| --- | --- | --- | --- |
| Stable Kujo release | 1.5.0 | annotated Git tag and GitHub Release | Yes |
| Rust crate/source package | 1.5.0 | `Cargo.toml` | Yes at a release commit |
| CLI | 1.5.0 | `CARGO_PKG_VERSION` via clap | Yes |
| REPL banner | 1.5.0 after fix | `CARGO_PKG_VERSION` | Yes; same binary |
| VM | no independent SemVer | same Rust package/binary | No separate version exists |
| Native archives | 1.5.0 | GitHub Release asset names + embedded CLI | Yes |
| Installer program | 0.1.0 | `KUJO_INSTALL_VERSION` in `install.sh` | No; installer implementation version |
| Installer default runtime | v1.5.0 | `DEFAULT_RELEASE_VERSION` | Yes |
| Setup action default | v1.5.0 after fix | action input default | Yes for current branch |
| npm source packages | 1.5.0 | six repository `package.json` files | Yes by current packaging design |
| npm published packages | 1.4.0 | npm registry dist-tags | Intended to match, currently blocked |
| crates.io package | unpublished | crates.io index | Not applicable until a publication policy exists |
| Language specification | 1.0.0 contract | `docs/LANGUAGE_SPEC.md` | No; stable contract baseline |
| CLI machine contract | 1.0.0 contract | `docs/CLI_MACHINE_READABLE_CONTRACTS.md` | No; changes only on contract break |
| Protocol contracts | 1.0.0 contract | `docs/PROTOCOL_CONTRACTS.md` | No |
| Runtime diagnostic contract | 1.0.0 | `src/errors.rs` | No |
| Workflow/doctor schemas | 0.1.0 where declared | schema constants/manifests | No |
| VS Code extension | 0.1.0 | extension `package.json` | No; separately packaged, unpublished |
| Tree-sitter package | 0.1.0 | grammar package metadata | No; separately packaged, unpublished |
| Kennel | 1.1.0 in current docs | Kennel's own release | No; separate repository/product |

Intentional differences are based on ownership and compatibility scope, not on
numeric similarity. In particular, `1.0.0` contract values must not be bumped
for every runtime release; `.github/scripts/check-contract-version-sync.sh`
deliberately locks them together.

## Meaningful `1.0.0` Classification

| Context | Examples | Classification | Action |
| --- | --- | --- | --- |
| Stable compatibility contracts | language spec, CLI JSON, protocol docs, runtime diagnostics | INTENTIONAL VERSION DIFFERENCE | Keep 1.0.0 |
| v1 launch evidence | v1 scope, official checklist, artifact checklist, changelog | NOT AN ISSUE | Preserve history |
| Stable v1 baseline docs | security posture, deprecation policy, AI runtime, standard-library baseline | INTENTIONAL VERSION DIFFERENCE | Keep unless the contract itself changes |
| Example/fixture metadata | MCP server versions, repo-gate schemas, sample package manifests | NOT AN ISSUE | Keep local fixture semantics |
| Research/design examples | Ability research implementation versions | NOT AN ISSUE | Do not infer runtime version |
| Active release-binary commands | former `KUJO_VERSION=v1.0.0` examples | STALE DOCUMENTATION | Updated to v1.5.0 |
| Dependency/spec URLs | SemVer 2.0.0, Keep a Changelog 1.0.0, licenses | Unrelated version | No change |

## Meaningful `1.4.x` Classification

| Context | Classification | Action |
| --- | --- | --- |
| Public npm runtime/platform packages at 1.4.0 | VERSION MISMATCH | Publish 1.5.0 after authorization is repaired |
| README/INSTALLATION statements that npm is 1.4.0 | Accurate current documentation | Keep until publication succeeds |
| “requires Kujo 1.4.0 or newer” compatibility floors | INTENTIONAL VERSION DIFFERENCE | Keep; minimum is not current release |
| September hardening evaluations and benchmark receipts | Historical evidence/test fixture | Preserve |
| Changelog/release history | Historical evidence | Preserve |
| v1.5 docs noting behavior absent in v1.4 | Compatibility reference | Preserve |

## E. Documentation Drift Report

### Current

- `README.md` after MSRV correction
- `INSTALLATION.md` after MSRV correction
- `ROADMAP.md` after current-line correction
- `docs/LANGUAGE_SPEC.md`
- `docs/STANDARD_LIBRARY.md`
- `docs/AI_RUNTIME.md`
- `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
- `docs/ARCHITECTURE.md`
- `docs/RELEASE_PROCESS.md`
- `docs/RELEASE_BINARIES.md` after example correction
- `docs/INSTALL_MATRIX.md` after registry-state correction
- `docs/ECOSYSTEM_INSTALL.md` after command correction

### Partially stale but now bounded

- `docs/CONCURRENCY.md`, `docs/MEMORY.md`, and `docs/EXTENDING.md` contain useful
  v0.9-era detail but are now labeled historical.
- `docs/VM_INSTRUCTIONS.md` is useful implementation material, not a stable
  instruction-set contract; it now says so explicitly.
- Legacy `.github` agent guides retain historical procedures but now defer to
  the root `AGENTS.md` and current roadmap.

### Clearly stale before this audit

- Setup action default/example at v1.2.3
- Agent guide's “stable v1.2.3” statement
- Roadmap's “current 1.4 line” statement
- Active release-binary examples at v1.0.0
- Ecosystem shared-ref and broken Dispatch manifest examples
- Rust 1.86 source-build prerequisite

### Unverifiable as current product claims

- Dated benchmark/evaluation reports and research notes are immutable evidence,
  not current release claims. They were excluded from normalization.
- Generated inventories and dependency lockfiles contain many unrelated version
  strings and were classified by provenance rather than edited.

## F. Broken Examples

No documented Kujo code example failed the repository's executable example
policy. `cargo test --test docs_examples` passed all six policy tests, including
tracked examples and top-level documentation fences. The downloaded v1.5.0
macOS x64 release binary also ran `examples/hello.kujo` successfully.

Two installation examples failed semantic/link verification rather than Kujo
execution:

| Source | Example/command | Actual result | Expected result | Cause | Fix |
| --- | --- | --- | --- | --- | --- |
| `docs/ECOSYSTEM_INSTALL.md` | installer with `--ref v1.2.3` | referenced ecosystem repositories lack that shared tag | install current compatible ecosystem | assumed synchronized repository tags | use installer default/product manifest |
| `docs/ECOSYSTEM_INSTALL.md` | Dispatch v1.1.0 manifest URL | HTTP 404 | downloadable reviewed manifest | stale tag/path | use v1.2.0 manifest |

The old Cargo upgrade example also failed package lookup because `kujolang` is
not published; it is recorded as a product-guidance finding above.

## Link Audit

Internal Markdown targets were checked separately from external URLs. Meaningful
broken internal links were the nonexistent concurrency example directory and
three removed helper-audit reports; all are corrected. Vendor documentation was
excluded. External current-doc checks found the stale Dispatch manifest; links
to placeholders, localhost examples, and intentionally illustrative domains
were not classified as failures.

## Release Notes Audit

The v1.5.0 tag contains the code and tests for each substantial release-note
claim inspected: `digest_file_beneath`, bounded `read_stdin`,
`list_dir_beneath`, `copy_file_beneath`, `sha256_file_beneath`,
`list_dir_page`, rooted prefix reads, isolated imports, capability gates, and
the associated race/boundary coverage. The release workflow and the later
published-artifact workflow both completed successfully across five targets.

Post-tag database and durability changes on `main` are correctly recorded under
`[Unreleased]`; they were not attributed to the v1.5.0 tag. No v1.5.0 claim was
found to be implemented only after the tag.

## G. Release Process Risks

1. External npm publication is not part of the same atomic success condition as
   GitHub release publication. A successful tarball-build job can coexist with
   a stale registry.
2. Current-release values are duplicated across Cargo, installer defaults,
   setup-action defaults, badges, documentation, and examples. The expanded
   release-state script now covers the known active locations, but a generated
   version inventory would be safer.
3. No crates.io package exists even though Cargo package metadata reserves the
   name. Any future registry claim must be verified after publication, not when
   packaging succeeds.
4. Historical design documents live beside current contracts. Status banners
   reduce confusion, but a documentation index with lifecycle metadata would
   make this distinction machine-checkable.
5. Link validation is not a mandatory release gate. A scoped internal-link test
   plus a non-blocking scheduled external-link job would catch recurring drift.

## Phase 2 — Fix Plan

### Commit 1 — Correct runtime identity, upgrade guidance, and MSRV

Files: `Cargo.toml`, `src/repl.rs`, `src/upgrade.rs`, corresponding test and
current source-build documentation, `CHANGELOG.md`.

Changes: declare/test Rust 1.89; derive the REPL version from Cargo; replace the
unpublished crates.io command.

### Commit 2 — Correct current release and installation documentation

Files: roadmap, install matrix, ecosystem install guide, setup action and guide,
agent guide, release-binary examples, historical-document banners, helper-audit
links.

Changes: remove stale current-release claims while preserving intentional
contract/history values.

### Commit 3 — Strengthen release drift automation

Files: release-state script and release-gate workflow.

Changes: verify Cargo/MSRV, installer, setup action, npm source manifests, and
active documentation; compile the lockfile at the declared MSRV.

### Commit 4 — Record audit evidence

File: this report.

Changes: preserve the classified evidence, commands, remediation, and remaining
external blocker.

## Phase 3 — Implementation and Verification

All confirmed repository fixes above were implemented. Historical changelog,
release checklists, evaluation receipts, fixtures, schema/contract versions,
dependency versions, and independent package versions were intentionally not
mass-edited.

Verification performed:

- `cargo fmt --check`
- `cargo +1.89.0 check --locked`
- targeted REPL/upgrade tests
- `cargo test --test docs_examples`
- `cargo test --test readme_contracts`
- `cargo test --test cli_contracts`
- `cargo test --test cli_json_contracts`
- `cargo test --test diagnostics_golden`
- package identity and package-boundary documentation tests
- `.github/scripts/check-release-state.sh`
- `.github/scripts/check-contract-version-sync.sh`
- `tests/install_release_manifest.sh`
- npm package tests and dry-run packaging
- scoped internal/external link checks
- v1.5.0 archive checksum, `--version`, and hello-example smoke

## Final Validation Answers

1. **Authoritative current release:** 1.5.0.
2. **Does `kujo --version` report it?** Yes: `kujo 1.5.0` in source and the sampled native release archive.
3. **Does Cargo metadata report it?** Yes: `kujolang 1.5.0`.
4. **Do published packages report intended versions?** GitHub native packages do; npm remains 1.4.0 and crates.io has no package.
5. **Does README describe the current release accurately?** Yes after the MSRV correction.
6. **Can a new developer follow installation instructions?** Yes for native archives, installer, and source builds; npm is explicitly limited to 1.4.0.
7. **Do documented examples work?** The executable Kujo example policy passes; the identified install examples were repaired.
8. **Are all `1.0.0` references understood?** Yes: contract/baseline, history, fixture/example, dependency/spec, or the corrected active binary examples.
9. **Are all `1.4.x` references understood?** Yes: public npm, compatibility floor, release delta, historical evidence, or changelog.
10. **Any remaining obsolete current-version references to change?** None found in active current-release surfaces after the repeat sweep.
11. **Legitimate independent versions?** Yes: contracts/schemas, installer, editor/tree-sitter packages, ecosystem products, and fixtures.
12. **Could release drift recur?** Yes, mainly at external npm publication and new unguarded documentation locations; risk is reduced but not eliminated.
13. **Exact changes now:** the four commit groups above, plus publish npm 1.5.0 after authorization is restored.
14. **Automation to add:** retain the expanded release/MSRV gates; add post-publish npm registry/install verification and scoped Markdown link validation.
