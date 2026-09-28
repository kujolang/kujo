# Kujo 1.6.0 publication record

Kujo **1.6.0 is released** on Linux x64/arm64, macOS x64/arm64 and Windows x64,
through native archives and the lifecycle-script-free npm runtime channel.

- Release: https://github.com/kujolang/kujo/releases/tag/v1.6.0
- Annotated tag: `v1.6.0`; exact source: `44af277848173664f72ca85f2a1b3b98d634ecdd`.
- Tag object: `7f5ec8380e086cac44e16df64a1a65cb1eb7fc84`.
- Runtime package: `@kujolang/kujo-runtime@1.6.0`, with five exact-version native
  optional dependencies. The public `latest` tag is `1.6.0`.
- No crate or participant SDK publication was performed.

```sh
npm install --global @kujolang/kujo-runtime@1.6.0
kujo --version
```

## Reviewed source and artifact continuity

The original authorization covered macOS x64 only. After that publication, the
operator explicitly expanded scope to all five supported native platforms and
runtime npm publication. The existing tag was not moved or rewritten. Its
annotation records the initial authorization; this record captures the expansion.
The original macOS x64 binary/archive, source archive, macOS x64 npm package and
neutral npm tarball retain their reviewed bytes. Four additional native targets
were built from the same source. Later release-tooling/docs commits are not the
artifact source or tag target.

The [RC provenance](evidence/kujo-1.6-rc/manifest.json) remains unchanged and retains
its historical unpublished/authorization state. It records the original local
runtime and full Dispatch/Workcell validation. The publication receipt is separate.

## Hosted verification

| Gate | Evidence | Result |
| --- | --- | --- |
| Exact-source full release gate and five native builds | [36463384911](https://github.com/kujolang/kujo/actions/runs/36463384911) | PASS; native upgrade tests, optimized binaries, npm pack tests |
| Exact archive regressions on five platforms | [36471655629](https://github.com/kujolang/kujo/actions/runs/36471655629) | PASS; 27 checks/platform, 135 total; historical/minimized loop return, closures, generators, async/tasks |
| Reviewed npm artifact reconciliation | [36471655728](https://github.com/kujolang/kujo/actions/runs/36471655728) | PASS; preserved reviewed macOS x64 and neutral tarballs |
| npm publication | [36471922585](https://github.com/kujolang/kujo/actions/runs/36471922585) | PASS; five native packages before resolver; signed npm provenance |
| Public native install and 1.5 → 1.6 upgrade | [36472191643](https://github.com/kujolang/kujo/actions/runs/36472191643) | PASS on all five platforms, including the setup action |
| Public npm clean install | [36472190906](https://github.com/kujolang/kujo/actions/runs/36472190906) | PASS on all five platforms with lifecycle scripts disabled |

The initial npm smoke attempt ran before registry processing completed and failed
with `ETARGET`. The initial Windows native smoke encountered GitHub's anonymous
API rate limit after successful installation/execution. Fresh failed-job reruns
passed without changing any runtime or package bytes. These initial failures are
retained in the publication evidence rather than counted as passing attempts.

The hosted full gate passed both 150/150 fixture sweeps, with eleven explicit
skips. Optional `cargo-audit` and `cargo-deny` were unavailable on that runner;
this is not a claim that those optional hosted checks ran. Four new published
native binaries used Rust 1.98.1; the retained macOS x64 binary used the reviewed
Rust 1.96.0 toolchain. Linux x64 used Ubuntu 22.04; Linux arm64 used Ubuntu 24.04;
macOS builds used macOS 15 runners; Windows used `windows-latest`.

## Installation defaults

The repository installer, setup action and current installation instructions now
select 1.6.0. The setup action includes Linux arm64. The website installer is
synchronized separately and its deployment/public-byte result is recorded in the
final publication receipt. Historical release references remain historical.

## Experimental boundaries

Wave C beta remains experimental, opt-in and restricted to its bounded
required/deny single-effect domain; alpha remains supported. Wave D handoffs and
participant SDK APIs remain experimental alpha under a trusted-local-host model.
Participant SDK packages remain private/unpublished.

No exactly-once, universal rollback, general machine-loss recovery, remote
participant trust, multi-effect assurance or stable participant SDK is claimed.
Source-blind agent adopter rehearsal passed. Human adopter usability remains
post-release validation and has not been claimed as performed.
