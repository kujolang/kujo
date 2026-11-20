# Kujo 1.10.0 final release readiness

## A. Executive verdict

**NOT READY.** The strict Rust dependency audit exits 1 on two timezone
maintenance findings. The checked-in VM/interpreter mismatch receipt also
exceeds the release gate's seven-day freshness requirement. Candidate CI,
five-platform package rehearsal and downstream acceptance still need exact-SHA
evidence. This review authorizes no official tag or publication.

## B. Repository summary

| Item | Verified state |
| --- | --- |
| Latest published release | [v1.9.0](https://github.com/kujolang/kujo/releases/tag/v1.9.0), 2026-10-22T19:37:44Z. Annotated tag object `07696adf641255f391c680059b1ecfe1f881e9e9` targets `2cfaf185ad03059c89ef4f72fb061074255d00fc`. GitHub reports a valid signature. Local Git lacked its SSH allowed-signers file. |
| Published distribution | GitHub exposes five native archives, the source archive and corresponding SHA-256 files. All six npm packages report latest 1.9.0. The crates.io sparse index returned 404 for `kujolang`. Marketplace returned 404 for `kujolang.kujo-language-tools`. |
| Audit baseline | Supplied `7a13207019a51e61d3db448bc13f59d5b0e332e5` is an ancestor of observed main `523c395d57cf939c14da806eaa1788fd785a71e4`, dated 2026-11-19T23:54:27-05:00. Local and remote main agreed at intake. The sole delta added 314 lines across six provider evidence documents. |
| Source history | `git rev-list --count v1.9.0..523c395d` returned 743 reachable commits, including 7 merges. First-parent count was 696. The full tree diff touched 1,303 paths. Squashed or rebased effects in main are included in the tree comparison. The historical commits removed by squashing must be assessed via branch evidence. |
| Reviewed candidate | `408c5ac4eec997530d2b9c20682715e7ae95cc90` is the last code/packaging change covered here. It has 749 commits and 1,320 changed paths relative to v1.9.0. This report and its indexes are a separate documentation commit. |
| Baseline hosted Rust suite | The full-gate step recorded 4,303 passing, 3 failing, and 28 ignored Rust test executions across 276 target result records. Three generated-inventory tests account for all failures in that log. Counts describe one invocation, with each test binary as its own target. |
| Candidate identities | Cargo crate `kujolang`, Rust library `kujo`, executable `kujo`, npm resolver and five npm payloads are 1.10.0 in source. Extension remains 0.1.0 with its own lockfile and release path. |
| Candidate distribution | npm runtime tests and npm dry-run packaging passed with controlled executable fixtures. Two deterministic source streams produced SHA-256 `abea4a536ecbb576d3dead2754a411d5a7367f994396ef995714d5fc47792769` at `408c5ac4`. The all-five real binary cohort has no completed rehearsal receipt. |

The [tag index](evidence/kujo-v1.10/tags.tsv) records 18 annotated tags, including 16 SSH signatures, one PGP signature and one unsigned tag. Beyond v1.9.0, signature format is recorded without revalidating historical signers. Published receipts for 1.7.0 through 1.9.0 remain in `release/`.

The [commit index](evidence/kujo-v1.10/commits.tsv) accounts for every reachable
post-1.9.0 commit at intake. It separates 203 commits touching runtime/tooling
source, 503 benchmark-or-evidence changes, 35 tests/release/dependency/example
or merge changes, and two documentation-only changes. Buckets describe paths
and may undercount functionality in mixed or independently replayed commits.
The 5.66 million added lines are dominated by benchmark evidence. The index
includes subjects, paths and overlapping subsystem categories for inspection.
Exhaustive per-commit semantic review of all source changes remains open.

The reviewed runtime chains include loop admission, bounded retry and no-JIT
fallback, physical-slot checks, captured collection publication and snapshots,
native call recovery, and VM-private native code lifetime. Relevant cases are
indexed by the [parity matrix](../VM_INTERPRETER_PARITY_MATRIX.md),
[ownership review](../NATIVE_OWNERSHIP.md) and
[performance consolidation](../../benchmarks/langarena/PERFORMANCE_CONSOLIDATION_2026_11_04.md).
The source census covered 3,543 tracked paths at intake. The search surfaced
996 marker, assertion or ignore matches on the selected source, test and
tooling paths. The regenerated strict triage classifies 25 source markers
with zero unclassified. They cover profiler wiring, JIT profiling and tail-call
follow-ups, and a type-diagnostic enhancement. Panic/sentinel controls and
expected-failure fixtures require their existing context. An exhaustive
line-by-line review of broader placeholders and all test-only assertions remains
open. The ignored suites include benchmark/resource probes, external GIF
conversion, LangArena checksum sweeps and Dispatch fixtures.

## C. Branch inventory

The [ref index](evidence/kujo-v1.10/branches.tsv) records 140 refs including the
symbolic origin alias, 54 live remote heads and 83 distinct tips. The recorded
remote heads matched their local tracking objects. Every ref has its full tip,
tip commit time, earliest available reflog entry, ahead/behind counts, merge
base, ancestry, unmatched patches and touched paths, PR states where found and
a cleanup recommendation. Reflog dates indicate the earliest surviving local
record. GitHub pull-request times and branch names cannot establish creation
time for branches without a matching record.

| Disposition | Refs | Action |
| --- | ---: | --- |
| Incorporated | 110 | Ancestral tips are eligible for owner review before deletion, subject to worktree and PR dependencies. Keep main and origin aliases. Preserve non-ancestor evidence branches pending any cleanup-specific equivalence review. |
| Patch-equivalent | 4 | Every branch-only commit has a patch match. Keep registered worktree refs. |
| Deferred | 14 | Keep seven proposals and their remote counterparts available. |
| Archive | 10 | Keep release/backport history. |
| Rejected | 2 | Keep the root-hash rejection and adverse control evidence. |
| Merge candidate or unknown | 0 | No merge recommended on the available branch evidence. |

The refreshed Git comparisons corroborate the [frozen branch decisions](../../benchmarks/langarena/V1_10_BRANCH_DISPOSITIONS_2026_11_04.md)
at unchanged proposal tips. Ancestry, cherry equivalence and source comparisons
cover integration identity. Direct tree comparisons also matched the memory platform candidate to its accepted source, the platform native implementation to intake main, and the retained local-store source to intake main. Original behavioral acceptance for retained
performance work is based on its pinned receipts. Those comparisons do not
retest the full criteria of each branch.

The [worktree index](evidence/kujo-v1.10/worktrees.tsv) lists 2 live clean worktrees
at intake and 15 inaccessible temporary registrations. Both historical dirty
draft patches pass `git apply --cached --check` with temporary indexes seeded
at their original HEADs. All four archived files, including the untracked
capture census source, matched their recorded SHA-256. The recovered drafts
belong in a disposable checkout at those original commits. Live directory
contents for the missing worktrees could not be compared.

## D. Confirmed findings

| ID | Severity | Subsystem | Finding | Evidence | Resolution | Status |
| --- | --- | --- | --- | --- | --- | --- |
| REL-01 | P1 | Dependency policy | Strict audit denies unmaintained chrono and chrono-tz on the candidate graph. | `Cargo.lock`, RustSec 2026-0543 and 2026-0555, audit exit 1. | Preserve strict policy. Review a maintained timezone and dependency path with date, timezone, PostgreSQL and PDF compatibility gates. | Open |
| REL-02 | P1 | Release infrastructure | Tagged builds could advance when Cargo audit was missing because the local gate treats it as optional. | Baseline `.github/workflows/release-binaries.yml` and `scripts/release_gate.sh`; new packaging regression fails against baseline. | Require the reusable strict audit job before artifact builds and enable benchmark smoke in the release gate. | Fixed in `37ef96d1` |
| REL-03 | P1 | Supply chain | Baseline audit recorded six vulnerability entries, including a critical capability dependency, and five denied warnings. | Baseline `Cargo.lock` and audit log. Candidate audit reports zero vulnerability entries and two warnings. A minimal link test measured SQLite 3.45.0 before and 3.54.1 after, failing and passing the 3.54.0 minimum respectively. | Update patched dependencies and adapt SQLite column iteration. Add a linked SQLite version regression and verify named result values in the database test. | Source fixed in `14b8ff5b` and `d1954b42`; strict candidate Clippy and the linked version probe passed. Full runtime regression pending |
| REL-04 | P1 | Release evidence | Checked-in generated inventories expired. | [Baseline Linux gate](https://github.com/kujolang/kujo/actions/runs/44677493704), three failures at `tests/generated_artifact_freshness_contract.rs:94`. | Regenerated the static unsafe and TODO inventories with strict checks. The runtime mismatch report still needs regeneration from the exact candidate executable. | Open |
| REL-05 | P2 | JIT assurance | Two executable JIT retirement boundaries lacked canonical pre/postcondition blocks. | `scripts/check_jit_safety_contracts.sh` reported 2 missing in 147 boundaries before `00a91811`. | Document owner and pointer lifetime guarantees and run the checker against the actual source in a regression test. No runtime instructions changed. | Fixed in `00a91811` |
| REL-06 | P1 | Editor integration | The real language client injected `--stdio`, which Kujo's default `lsp` command rejected. | VS Code 1.143.1 isolated extension-host logs recorded rejected argv and restart exhaustion. | Use the client's default stdio transport without a transport switch. Return the startup promise to VS Code and register the file watcher as a disposable. A host smoke test requires a real LSP hover response. | Fixed in `408c5ac4` |
| REL-07 | P2 | Windows JIT | VM-owned code memory can remain allocated under the pinned Windows backend. | [Platform acceptance record](../../benchmarks/langarena/PERFORMANCE_CONSOLIDATION_2026_11_04.md). Current Windows reclamation was unavailable locally. | Keep the limitation explicit. Reassess long-running repeated-VM workloads on Windows before making wider reclamation claims. | Disclosed |
| REL-08 | P2 | Publication recovery | Manual npm recovery takes a run ID and version. It validates package counts, versions and registry readback integrity, while depending on the owner to select the reviewed tagged source run. | `.github/workflows/publish-npm-runtime.yml`. GitHub and npm publication jobs are independent, so a red run can have partial public results. | Require the owner to reconcile the signed tag, workflow run SHA, cohort and integrity before any recovery dispatch. | Open operational check |

## E. Test and CI matrix

Local commands ran on macOS 26.6.2 x86_64, Rust/Cargo 1.96.0, Node 26.12.1
and npm 11.20.0 unless specified. Raw logs, observed exits, environment and
source identifiers are in `/tmp/kujo-v110-final-523c395d/` on the audit host.
Key text logs are also retained in the ignored local directory `.audit-evidence/kujo-v110/`.
Links identify hosted evidence. Pending runs cannot substitute for a terminal
pass at the source selected for release.

| Command or workflow | Source and environment | Result and coverage |
| --- | --- | --- |
| `cargo fmt --check`; strict Clippy | Intake macOS | Both passed. Local full gate continued to compilation and was stopped to preserve remaining disk. |
| Baseline full hosted gate | `523c395d`, Linux x64, [44677493704](https://github.com/kujolang/kujo/actions/runs/44677493704) | Clippy, MSRV, minimal gate, format and selected parity passed. Full gate failed on 3 freshness tests, 4,303 pass / 3 fail / 28 ignored. Focused commands later in the shell gate were skipped after exit 101. |
| Baseline filesystem, database and artifact flow | `523c395d`, [filesystem](https://github.com/kujolang/kujo/actions/runs/44677493892), [databases](https://github.com/kujolang/kujo/actions/runs/44677493807), [artifact flow](https://github.com/kujolang/kujo/actions/runs/44677493709) | Successful at intake. They do not cover the subsequent lockfile and extension changes. |
| `KUJO_ENABLE_SOCKET_TESTS=1 KUJO_RELEASE_GATE_RUN_BENCH=1 bash scripts/release_gate.sh --full` | `9692b974` dependency/version candidate, macOS | Clippy reproduced the updated rusqlite column iterator API mismatch. The later correction changes the iterator call and verifies column aliases. Socket and benchmark stages were not reached. |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | `408c5ac4`, macOS x64 | Exit 0 after the rusqlite iterator correction. No behavioral test results are implied. |
| `cargo test --locked --no-fail-fast`, sockets enabled | Candidate transition, macOS | Interrupted during dependency compilation when remaining disk approached 300 MiB. The later targeted database test and local executable build at `408c5ac4` were interrupted at about 205 MiB free. These attempts produced no runtime test count. |
| `cargo audit --deny warnings` | Intake, then 1.10.0 lockfile | Intake failed with 6 vulnerabilities / 5 warnings. Candidate failed with 0 vulnerabilities / 2 warnings. See REL-01. |
| `cargo deny check` | macOS | Exit 101, tool missing. Latest upstream release has no Intel macOS prebuilt binary. Run the check on a capable runner and preserve its exit/log. |
| Release packaging and safety checker regression tests | Local standalone Rust test harnesses against the edited tree | 4 packaging tests and 9 safety checker tests passed. Packaging audit requirement failed against the baseline workflow. The strict JIT checker now reports 147 boundaries with none missing. |
| Version, roadmap and lock metadata | Local candidate | Tag, release-state and roadmap-only guards passed. Cargo 1.89 manifest metadata passed. Offline full metadata lacked a newly locked Windows crate; compilation with MSRV remains a separate gate. |
| npm package contract and dry run | Local candidate, Node/npm | 12 tests passed, and all six package identities packed using synthetic native fixtures. No native binary claim. |
| VS Code dependency and actual host | Clean extension checkout, macOS, VS Code 1.143.1 | `npm ci`, audit with zero advisories, 5 dependency tests and VSIX packaging passed before the LSP launch fix. Isolated host reproduced the baseline argument rejection. The corrected host produced the successful hover marker using the available local runtime. The corrected VSIX packaged and installed into an isolated profile as `kujolang.kujo-language-tools@0.1.0`. Candidate-exact host retest is still required. |
| ShipCheck scan | Intake, ShipCheck 1.0.2 | Exit 0, 12 of 16 checks, 4 warning-level heuristics for Rust lint/format and Kujo manifest/entrypoint. Its metadata gate passed. |
| Candidate native cohort workflow | Branch manual dispatch with input `version=v1.10.0` | [Previous dispatch 44680186544](https://github.com/kujolang/kujo/actions/runs/44680186544) failed in the dependency audit and all packaging and publication jobs were skipped. [Candidate dispatch 44680652075](https://github.com/kujolang/kujo/actions/runs/44680652075) failed at the same required audit job on `408c5ac4`, with native, source, npm and publication jobs skipped. The separate [candidate CI gate](https://github.com/kujolang/kujo/actions/runs/44680620558) was still queued at the last snapshot. |
| Clean-host upgrade verification | Five target matrix | Pending on all five targets for the exact candidate. Baseline installs and upgrades are historical published evidence. |
| Stress, no-JIT, repeat checks | Linux hosted intake or prior campaign | Selected no-JIT parity passed at intake. Candidate socket stress, repeated async/callback/VM destruction and updated lockfile cross-platform regressions remain pending. |

## F. Performance acceptance

The retained checkpoint includes bounded loop admission/retry, physical-slot
identity, capture and dictionary/array batching, named/field reads, depth cache
hashing, VM-private JIT lifetimes and consuming SSA local stores. Historical
Linux CI 41707246908 recorded two independent sessions with 1,321 library passes,
89 selected integration passes and 150 exact checksum results each. CI 41717910054
also checked complete sort arrays, wrapper observations, Neural state, exact
float and pixel results and resource controls. Sorting A/A/B controls are
recorded in 41719214681. Those runs establish specific source/workload evidence.
They cannot substitute for repeating matched baselines after updating the
candidate toolchain or dependency graph. No fresh performance gain is claimed.

The working-tree review retains these deferred decisions.

| Proposal | Reason to keep outside 1.10 |
| --- | --- |
| JIT owner reuse, `1eea508d` | VM Neural regression and incomplete resource/platform acceptance. |
| Native implementation, `49c0d989` | Local stores are retained through the accepted checkpoint. Later scalar indexed writes need an independent second timing session, complete state/resource checks and disposition of adverse VM Quick/BWT controls. |
| Native field fusion, `53b19056` | One promising timing session. Independent timing, Neural state/resource and combined-source acceptance are incomplete. |
| Kennel tool import isolation, `66029f26` | The isolation/type fixes are incorporated. Remaining fuzz pinning must be validated on the current toolchain and locks. |
| crates.io HTTP dependency, `23eb4ca2` | A reviewed patched registry identity and normalized package verification are missing. Keep the vendored implementation. |
| Main-only scalar loading, `03603253` | A reproducible workload-level benefit has yet to be established. |
| String-cache dispatch, `7e07db7f` | Inconsistent benefit and unresolved cold costs. |
| Root hash, `04a9321e` | **Rejected.** Repeated uncached-read and Merge VM regressions. Preserve negative controls. |

## G. Security assessment

The fixed RustSec entries concern capability confinement, constant-time
operations, bundled SQLite, compression build/runtime issues and TLS
verification. Updated versions include cap-primitives 4.0.3, ctutils 0.4.4,
libsqlite3-sys 0.39.1, libz-sys 1.1.33 and rustls-webpki 0.103.16. Rustix and
Tokio unsound warnings and the yanked Windows clipboard package also clear.
The two remaining notices concern maintenance. The audit does not report
specific runtime exploitation of those warnings. Review direct `chrono-tz`
and local time paths as well as PostgreSQL and PDF dependencies before
replacing the stack.

Inspection emphasized capability dispatch, descriptor-relative filesystem
operations, process lifetime, HTTP limits, native pointer ownership and
distribution boundaries. The project documents separate capability controls,
host isolation and Windows runtime behavior. Strict JIT contract checks and
lexical inventories are assurance evidence. They do not establish memory
safety, exploitability, or exhaustive path coverage. The vendored tiny_http
patch remains selected for repository builds. `cargo package --list` verifies
the file list. Normalized registry-build parity for vendored patches remains
unverified. VS Code uses a guarded hash-checked patch against the pinned
language client and keeps the minimatch correction. The registry latest client 11.0.0 still requests minimatch 10.x. The advisory range includes 10.2.6, so that release does not remove the reason for the guarded patch. The host launch fix
addresses a separate interface defect.

## H. Compatibility assessment

The existing VM and interpreter tests cover indexed writes and snapshots,
call/recovery, imports, async, capabilities, native data representations,
serialization and fixtures. Baseline Linux CI demonstrated 123 tests in the
parity surface target. The optimized campaign covers JIT entry/fallback and
retention boundaries. Exact-source combined results with the dependency edits
and all supported platforms are absent. SQLite must preserve row aliases,
types, close/transactions and read-only paths after its dependency update.
The bundled engine was advanced for the upstream WAL fix.

The local first-party intake recorded the following checkout revisions and
interfaces. These are source and contract observations. The candidate
executable did not run their current-head acceptance suites as part of the
completed evidence.

| Consumer | Intake SHA prefix | Integration or verification requirement |
| --- | --- | --- |
| Dispatch | `18b5c737` | Kujo workflow entrypoints, resource and failure-gate contract. |
| Workcell | `189d7ab6` | Runtime/OCI lifecycle, startup and reporting contracts. |
| Spec | `36da0d72` | Manifest-driven CLI/schema and exit contracts. |
| Eval | `c1b263a1` | Fixture runner and artifact/CLI matrix. |
| RunLedger | `ab0ec5d2` | Runtime CLI, deterministic receipts and process exit semantics. |
| Casefile | `92e1d9be` | Kennel-installed runtime tooling, path and failure evidence. |
| ShipCheck | `7eedd9e0` | Runtime command and JSON gate output. Local scan used the available runtime. |
| Watchdog | `7a0bd9a6` | HTTP, streams and SQLite. Checkout contained unrelated changes. |
| Lens | `91049646` | Kujo tooling plus Node bridge and package install. Checkout contained unrelated changes. |
| MCP | `77180a3a` | Kujo scaffolding and JSON-RPC/tool contracts. |
| AI SDK | `c2223914` | Offline provider and replay contracts. |
| Agents SDK | `70cd1fb7` | Offline execution, tool/approval and runtime effects. |
| Kennel | `6e2fc205` | Package installation, imports and registry/tool contracts. |
| Official CLI tooling | `7dedc530` | Node distribution and installed tool integration. |
| Public MCP catalog | `b0713960` | Catalog adapter and controlled promotion. |

Hosted release CI uses separate pinned fixtures for AI SDK, Agents SDK, Eval,
MCP, RAG, Dispatch, RunLedger, Watchdog and Relay. The workflow pin is the
identity for those tests. Do not substitute the intake checkouts for that pin
or treat a previous-pin pass as current-head acceptance.

## I. Release preparation

| Commit | Files and purpose | Regression and compatibility |
| --- | --- | --- |
| `37ef96d1` | Release and audit workflows, release process, packaging tests. | Publication artifacts require a green audit. Standalone packaging test reproduces baseline absence and passes with the change. |
| `14b8ff5b` | Cargo manifests, database test and changelog. | Bounded lock updates and linked SQLite minimum check. The audit removes all six vulnerability entries. |
| `9692b974` | Cargo, npm cohort, RC JSON, README, roadmap, changelog, candidate notes and candidate identity test. | Version guards and npm dry run passed. Windows ZIP identity is explicit. Stable installer and published v1.9.0 receipts are retained. |
| `00a91811` | JIT safety documentation, checker regression and regenerated unsafe/TODO inventories. | Strict checker and all 9 checker tests pass. No executable JIT change. |
| `d1954b42` | SQLite native API and existing unit test. | Corrects the compile-time iterator API mismatch. Strengthens the observable result test to require exact alias names and typed values. |
| `408c5ac4` | VS Code extension, host regression and extension README. | Isolated host reached real Kujo hover after the argv correction. Extension publication remains separate. |

Planned assets are `kujo-v1.10.0-{linux-x64,linux-arm64,macos-x64,macos-arm64}.tar.gz`,
`kujo-v1.10.0-windows-x64.zip`, `kujo-v1.10.0-source.tar.gz`, per-file
SHA-256 sidecars and `checksums.txt`. The npm resolver plus five exact platform
package identities are in [candidate metadata](../../release/kujo-1.10.0-rc.json)
and the [npm workspace](../../npm/README.md).
[Candidate release notes](../RELEASE_NOTES_1_10_0.md) organize the proposed
change and compatibility claims. Historical receipts remain at 1.9.0.

## J. Remaining blockers

1. Resolve or obtain an explicit owner policy decision on the two strict
   RustSec failures. Current release policy requires a successful audit.
2. Regenerate `VM_RUNTIME_MISMATCH_INVENTORY` with an exact candidate binary and
   pass the generated artifact freshness tests with their existing assertions.
3. Complete the exact-source full gate, opt-in sockets and benchmark smoke,
   separate locked test run, Clippy, MSRV build and `cargo deny check`.
4. Finish all five native builds, SHA-256 checks, clean-host installation,
   upgrades, npm wrapping and first-party acceptance at recorded revisions.
5. Complete source/patch review and fresh matched performance measurements
   where required for release assertions. The bounded Windows JIT limitation
   remains explicit.

## K. Final authorization checklist

- [ ] Select the clean candidate SHA. Run `cargo fmt --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --locked --no-fail-fast`, `cargo audit --deny warnings`,
  `cargo deny check`, and
  `KUJO_ENABLE_SOCKET_TESTS=1 KUJO_RELEASE_GATE_RUN_BENCH=1 bash scripts/release_gate.sh --full`.
- [ ] Verify generated artifacts using
  `bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict --runner target/debug/kujo`
  after building that runner from the same source. Run
  `cargo test --locked --test generated_artifact_freshness_contract`.
- [ ] Run `KUJO_RELEASE_TAG=v1.10.0 bash .github/scripts/check-tag-version.sh`
  and `bash .github/scripts/check-release-state.sh`. For prepublication
  five-target artifacts run
  `gh workflow run release-binaries.yml --ref main -f version=v1.10.0`
  and confirm its head SHA before reviewing its artifacts. This command's
  branch dispatch uses the nonpublishing path.
- [ ] Review the release notes, downstream pins, check results, Windows
  memory limitation, executable identity, upgrade receipts and source checksum.
  Verify GitHub environment and npm trusted-publisher configuration using
  owner access without printing credentials.
- [ ] Only after explicit publication authorization, follow
  [release process §8](../RELEASE_PROCESS.md#8-tagging-and-publication-order).
  The owner may sign the reviewed SHA with
  `git tag -s v1.10.0 <approved-sha> -m 'Kujo v1.10.0'` and push it with
  `git push origin refs/tags/v1.10.0`. Monitor both GitHub and npm publication
  jobs, then run the published native and npm smoke workflows.
- [ ] After verifying both published channels, reconcile the public installer,
  website, documentation and ecosystem consumers per the release process.
