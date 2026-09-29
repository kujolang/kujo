# Kujo 1.6.0 release candidate review

**KUJO_1_6_RC_READY**, limited to the built/tested macos-x64 candidate. Nothing
was tagged or published. Other platform artifacts require their own gates before
a multi-platform release claim.

## Source, version and tag target

- Exact tested source / intended `v1.6.0` target: `44af277848173664f72ca85f2a1b3b98d634ecdd`.
- Baseline: `50d68997ec7cbd93440776ff6ba7d12d802d991d`; includes regression `2fe6ed3` and causal fix `db4608c`.
- Cargo.toml, Cargo.lock's kujolang entry, CLI, npm workspace/runtime/five native
  manifests, README source badge/statement, ROADMAP crate line and candidate
  metadata agree on **1.6.0**. Language/CLI/LSP compatibility versions remain 1.0.0.
- Published installer defaults remain **1.5.0** deliberately: no 1.6 artifacts have
  been published. Historical release/evidence pins are not current-source pins.
- This evidence receipt is committed after the tested source. It must not be used
  as the tag target. The archive is an exact `git archive` of the source above.

[Version classification](RELEASE_1_6_TRUTH_AUDIT.md) and the compressed occurrence
inventory preserve the audit. CHANGELOG has one Unreleased section and an explicit
unpublished 1.6 candidate section. Historical release entries are preserved; only
the separator before a removed trailing duplicate Unreleased section was trimmed.
[Release notes](RELEASE_NOTES_1_6.md) distinguish runtime behavior from companion
repositories and their experimental contracts.

## Runtime gates and loop/return correctness

- Full canonical release-candidate gate passed again on the final source:
  fmt, all-feature Clippy, **2,857 Rust tests**, 17 existing ignores, 95 suites,
  focused security/package/parity contracts and 31 socket serve tests.
- `cargo check --locked` passed; **233 targeted contracts** passed.
- VM, interpreter and dual each **150/150**, 11 existing skips, **zero dual fallback**.
- Original retained historical probe and ten minimized loop/return cases passed
  under the final optimized binary in both runtimes. Caller results, no-match,
  later-match, nested branches/loops, folding and break/continue remain covered.
- Release-state, tag-version dry-run and compatibility-contract guards passed.
- cargo-audit passed under the existing build-only RUSTSEC-2025-0141 exception.
  Optional cargo-deny was unavailable. Benchmark smoke was not enabled; no new
  performance claim is made. No hosted CI was observed.

## Built binary and archives

`cargo build --release --locked` from clean source, `rustc 1.96.0 (ac68faa20 2026-05-25)`,
`cargo 1.96.0 (30a34c682 2026-05-25)`, platform `macos-x64`.

- Binary SHA-256: `930e0da1fec2562f6990d1226a479330640c8c78eb4c36c148cee3f950b29a47`.
- Cargo.lock SHA-256: `26061df9462d1648acc59ce38380ab34bdc3253ad4c66008f125c7793dcdc7ac`.
- `kujo-v1.6.0-macos-x64.tar.gz`: `ee8aad0b389256f0d841891b97f6aa0826027f2f70894bb240f82599a678d317`.
- `kujo-v1.6.0-source.tar.gz`: `21379f60f63e638c4e126fe9bc2bb30c56732315ca0118be73016046d6edddd7`.

Additional locally packed runtime npm artifacts (not participant SDK publication):
- `kujolang-kujo-darwin-x64-1.6.0.tgz`: `2375c8a644047821a5137a03a5a05cadb0f7b56c88001a8c4cd6ce89b232c258`.
- `kujolang-kujo-runtime-1.6.0.tgz`: `b62ea8a8e0f2e113f721a25eb873cd5f224b4ad86128e25b09d3a1b21fc551bc`.

Retained artifacts: `/Users/robertdevore/2026/Kujolang/release-candidates/kujo-1.6.0-44af277`. Native archive format matches the
release workflow (binary-only tar.gz plus checksum). Native tar timestamps are
retained; byte-reproducibility is not claimed. Source archive uses `gzip -n` and its
decompressed bytes exactly match Git's archive output for the tested source.
Linux x64/arm64, macOS arm64 and Windows x64 were not built here.

## Clean installation

The archive was extracted to a new temporary bin/home/test directory. With a
minimal environment and no source-tree runtime path, version was exactly
`kujo 1.6.0`; 15 programs passed VM and interpreter (30 executions), plus 15/15
dual with zero fallback. These include the original/minimized loop probes,
closure, generator, simple program and eager async/reusable task waits.
The first harness run caught a missing terminal newline in the existing closure
golden file. Normalizing only that golden terminator fixed the comparison; both
runtimes already agreed, and no runtime or archived fixture was changed. The
initial failure log and correction are retained.
The runtime and native npm tarballs also installed offline into an empty consumer
using `--ignore-scripts`, with no registry fallback; its CLI reported 1.6.0.

## Downstream reconciliation

Full Dispatch release gate passed against the **extracted exact RC binary**:
45 focused suites, 24 shards / 101 source contracts, command smoke and three
release workloads. This includes Wave C schemas/profiles/compatibility/persisted
negotiation/beta migration, SDK/MCP/HTTP/Git, native TypeScript/Python, generic
core, SDK source-package/conformance checks and legacy failure/reexecution/review.
Prior SDK packaging/private-distribution evidence remains historical; this gate
does not independently rerun every private-feed substitution test.
The automated installed-package adopter integration was additionally rerun:
pre/post-CAS SIGKILL retained unknown knowledge; live states were not_started and
committed respectively; fresh controllers admitted the reviewed continuation;
one logical effect remained. Contention was one admitted / three denied.
The source-blind onboarding exercise itself was not repeated.

Workcell full canonical `tests/run.sh` and effect-assurance gate passed. Its
`WORKCELL_TEST_KUJO_VERSION=1.6.0` is an exact executable-version test override;
released 1.2.1 pins and minimum compatibility floor remain historical, unchanged.

| Repository | Tested source |
|---|---|
| dispatch | `955996b83a5e3cf416eb6fbacc05788faffdbc29` |
| workcell | `1940da0639b70b702c1b1077dda51ca39b065216` |
| ability | `d6c970785f8d8bea04de0dce37920c2d0ca1c067` |
| mcp | `a7ec0dd8e6bcae303ab1431b4586dfe3e91f3a5a` |
| agents-sdk | `af0aa28f5960232cbafa7cf528a32db8cb36c7a9` |
| ai-sdk | `71bad1468fbc97eab830a27c185c032f07fd76cf` |

Dispatch's new `release/kujo-1.6.0-rc.refs` and active CI runtime/source checks select
this candidate. Its historical dispatch-v1.3.0 refs remain unchanged. Supporting
CI cohorts are documented, not silently repinned or claimed as hosted RC results.
Preexisting Agents SDK maintenance files remain untouched and outside the pinned
source cohort; no new maintenance code was used by the bounded integration.

## Stable versus experimental

Wave C remains beta, experimental opt-in required/deny single-effect with alpha
retained. Wave D correlation and participant SDK APIs remain alpha, unpublished,
local trusted-host. No replay/admission authority moved from Dispatch. No
exactly-once, universal rollback, general machine-loss recovery, remote participant
trust, multi-effect assurance or stable SDK guarantee is made.
Source-blind **agent** rehearsal passed; human usability remains unperformed
post-release validation.

## Provenance and commit map

[Machine-readable receipt](evidence/kujo-1.6-rc/manifest.json), schema
`kujo.release-candidate-provenance/v1`, contains exact source/toolchain/lock/binary/
archive/pin identities, gate counts, commands, log hashes and platform limits.
Its adjacent SHA-256 file checks the receipt. Compressed raw logs and all Dispatch
suite logs are retained, not just aggregate success statements.

| Repository | Commit | Purpose |
|---|---|---|
| Kujo | `b14951e40f84830188a7065391b962a98fc8232c` | Canonical 1.6 metadata and version regression |
| Kujo | `44af277848173664f72ca85f2a1b3b98d634ecdd` | Changelog/notes/truth audit and artifact builder; tested RC/tag target |
| Dispatch | `955996b83a5e3cf416eb6fbacc05788faffdbc29` | Current-source RC cohort/pins, historical closure retained |
| Kujo | Subsequent receipt commit containing this review | Gate/artifact provenance only; not the artifact source |

## Release checklist

Complete locally:

- [x] Source/version/changelog/docs reconciliation and strict metadata checks.
- [x] Exact optimized build, native/source/npm archives and checksums.
- [x] Clean installs, original runtime regression and full Kujo/Dispatch gates.
- [x] Workcell compatibility/effect assurance and automated adopter replay.
- [x] Stable/experimental and human/agent evidence boundaries explicit.

Outstanding at the time of this historical RC review (see the current
[1.6 release notes](RELEASE_NOTES_1_6.md) for shipped status):

- Run remaining platform builds/gates before claiming multi-platform support.
- Create/push `v1.6.0` at the exact tested source above (never receipt HEAD).
- Create GitHub release and upload reviewed archives/checksums/provenance.
- Authorize runtime npm/crate publication if desired; participant SDKs remain
      a separate unpublished experimental distribution decision.
- Reconcile public install defaults and latest-published statements only when
      publication actually exists; verify downloaded artifacts afterward.

Post-1.6: human adopter usability; remote trust threat model; A2A; stable SDK;
assurance renewal/broader beta policy/multi-effect design; broader performance
characterization. None is represented as a current runtime guarantee.

Next bounded task: authorize the intended 1.6 release scope/platforms, complete
remaining platform gates, then perform the explicit tag/upload/publication and
post-publication install-default reconciliation against these reviewed identities.
