# Repository hardening — 2026-10-19

## Repository

- Repository: `kujolang/kujo`, branch `main`; initially clean.
- Starting SHA: `27be6e55247e071509f8791799e448ed55659dc6`.
- Ending implementation SHA: `843e1df007ef5bfc99271b0fbc79fd3b04a74797`.
  The subsequent documentation commit contains this receipt and measurements.
- Purpose: Rust-hosted Kujo 1.8.1 runtime, VM/interpreter, CLI/LSP, native host
  effects, local package contracts and deterministic AI primitives.
- Integrations: Tokio, OpenSSL/reqwest, SQL providers, image/PDF, Cranelift,
  ZIP/TAR, local Kennel installations, agent SDKs and GitHub/npm releases.

This is a risk-oriented engineering pass with independent architecture, baseline
security and focused archive reviews. It is not an exhaustive line-by-line audit
of all 2,008 files. It supplements the October 9–15 reports without treating their
conclusions as proof of the current implementation. No sibling source is changed.

## Baseline

Host: macOS x86_64. Formatting passed before edits. Full Cargo baseline was
started before production changes. The first dependency build was interrupted to
switch to nonincremental, single-job compilation because available disk space
was roughly 4–6 GiB. Logs are retained in the ignored repository-local directory
`.audit-evidence/hardening-2026-10-19/`.

Dependency audit used RustSec database commit
`999ec89d5d3f10f20be14750ec8c2d8b44e0ac9b` (1,338 advisories; 652 locked packages).
It failed on three ZIP advisory matches and maintenance warnings for
`unsafe-libyaml`, `zbus_names`, `zvariant`, and `zvariant_derive`. The existing
build-only `RUSTSEC-2025-0141` exception was retained; no new suppression added.

Full-suite and regression outcomes are recorded in the verification receipt below.

## Findings

| ID | Priority | Area | Finding | Evidence | Action | Status |
| --- | --- | --- | --- | --- | --- | --- |
| H01 | P2 | Archive confinement | Existing outside ancestor aliases permit directory creation before rejection | `filesystem.rs`, `extract_zip_archive_with_limits`; two independent traces | Validate existing ancestors before directory creation; preserve in-root aliases | Reproduced before; targeted tests pass after |
| H02 | P2 | Module state | Initializer source replacement stamps old exports with new metadata | `module.rs`, final cache insertion | Admit cache only when metadata remains equal across evaluation | Reproduced before; targeted tests pass after |
| H03 | P2 | Import efficiency | Every uncached import appends its parent again, multiplying search-root cloning and lockfile discovery | `module.rs`, initializer loader transfer | Deduplicate roots while retaining discovery invalidation | Implemented; measurement below |
| H04 | P2 | Dependencies/API | Locked ZIP 7.2.0 matches three new upstream advisories; ZIP 9 changes an exposed Rust writer type | RustSec 2026-0367/0368/0369; `value.rs:1335` | Preserve public Rust compatibility; require a deliberate dependency migration | Open; affected upstream extraction APIs are not called here |
| H05 | P2 | Maintenance | YAML and Linux credential-store transitive crates are unmaintained | RustSec maintenance warnings | Preserve gates; migration requires compatibility work | Open |
| H06 | P2 | CI efficiency | Two current-source Windows jobs build unused OpenSSL through vcpkg before Cargo builds its vendored copy | Cargo.toml and openssl-sys 0.9.117 build selection | Remove redundant setup from native-upgrade and release-binaries | YAML parsed; native platform CI pending |
| H07 | P2 | Baseline verification | VM mismatch inventory exceeded its seven-day freshness budget | Baseline full suite: nine days old, generated 2026-10-10 | Regenerate through the existing strict generator and inspect differences | Regenerated; only date changed |

## Changes implemented

- **Archive paths:** a shared helper locates the nearest existing ancestor and
  checks its canonical containment before either directory-entry or file-parent
  creation. Dangling/unresolvable aliases fail before creation. Existing final
  symlink rejection, staging, permissions and decoded-size limits remain intact.
  `tests/archive_ancestor_confinement.rs` reproduces both outside-tree mutations
  on the original runtime and verifies unchanged sentinels, no outside-created
  directories, and positive in-root aliases in VM and interpreter. The existing
  filesystem platform workflow now includes this target (Unix alias cases are
  explicitly Unix-only).
- **Module cache correctness:** retain the pre-load metadata and admit evaluated
  exports only if metadata still matches after initialization. A self-replacing
  initializer returns its current evaluation but cannot poison the next import
  with stale exports labeled as the replacement source. Public regression tests
  also verify unchanged source initializes once and explicit re-registration
  refreshes newly installed locked packages.
- **Module resources:** independently deduplicate exact path entries in both
  ordered search-root vectors; preserve their first occurrence and unconditional
  discovery-cache invalidation. This removes growing duplicate storage and work
  without introducing a new cache policy. A 32-module resource regression asserts
  root counts remain fixed; the public microbenchmark checks every export.
- **CI:** remove two redundant vcpkg setup steps from current-source Windows jobs.
  Enabled OpenSSL features and Cargo's build remain unchanged. YAML parsing is
  validated locally; no new native Windows execution claim is made.
- **Generated records/docs:** strict generators refresh the dated VM inventory
  and the unsafe inventory's shifted source anchors. Counts/classifications and
  mismatch rows are unchanged. Canonical language/security docs and CHANGELOG
  describe the corrected behavior; no assertion, timeout or ignore was weakened.

## Performance and efficiency

The import benchmark imports distinct small modules from one directory, checks
every returned value, excludes fixture creation, performs two warmups and seven
samples, and uses identical compiler flags for baseline and candidate. Source:
`scripts/module_load_bench.rs`. No noisy CI timing threshold is introduced.

The same driver was linked before and after the implementation against the
corresponding default-feature library, using Rust 1.96.0:

```sh
rustc --edition=2021 scripts/module_load_bench.rs \
  --extern kujo=target/debug/deps/libkujo-9893f0e4ff9490c4.rlib \
  -L dependency=target/debug/deps -o .audit-evidence/hardening-2026-10-19/module-before
# Repeat with the candidate library and output name module-after.
```

The library filename is this host's Cargo artifact hash, not a portable constant.
Both executables are unoptimized. Three alternating pairs run after builds/tests
finish; the summary is the median of three invocation medians. An initial timing
under build contention is excluded from the final comparison. Every run verifies
the same exports; timings concern distinct imports, not cache-hit throughput.

| Distinct modules | Before median | After median | Elapsed-time reduction | Speedup |
| --- | --- | --- | --- | --- |
| 100 | 341.05 ms | 153.28 ms | 55.1% | 2.22× |
| 500 | 5,560.83 ms | 791.86 ms | 85.8% | 7.02× |

Raw samples and exact medians are committed in
[`repository-hardening-2026-10-19-benchmark.json`](repository-hardening-2026-10-19-benchmark.json).
All six invocations completed and verified exports. The root-count regression
separately proves bounded duplicate storage without relying on timing. These are
local unoptimized microbenchmark results. No application-wide throughput,
provider token, RSS, build-time or binary-size improvement is claimed.

## Review coverage and decisions

- **Contracts/frontend:** read README and agent guidance, source limits, import
  semantics, CLI capability/error construction, package manifest/lock serialization,
  tests and CI. Public names, schemas, exit codes and trusted-default behavior are
  compatibility requirements, not cleanup targets.
- **Complexity/state:** traced module cache admission, nested loader transfer,
  search-root precedence and lazy Kennel discovery. No broad VM/JIT rewrite or
  removal of established public helpers is justified. Metadata-based freshness
  remains distinct from content/dependency-aware hot reload.
- **Runtime/resources:** reviewed archive decoded-size staging, recursive archive
  creation, bounded upgrade extraction, async runtime ownership and worker stack
  allocation, filesystem confinement helpers, native limits and process lifetime
  contracts. No new sandbox or whole-process memory guarantee is implied.
- **AI/context/output:** token fitting borrows messages, validates every message,
  estimates once and selects linearly; existing limits and normalized return shape
  remain intact. Release-gate logging already supports full file evidence plus
  concise receipts. Avoided duplicate output frameworks, speculative caches and
  agent-instruction rewrites.
- **Failure handling:** examined extraction failure mutation, staged publication,
  module evaluation errors and loading-stack cleanup. Existing archive partial
  success across entries remains documented; no archive-wide transaction claimed.
- **Security:** reviewed capability policy, filesystem/ZIP paths, compression
  bounds, network DNS/redirect controls, AI endpoint allowlisting, agent credential
  resolution, static-file selection, upgrade integrity and release authority.
  Example/test keys have explicit dummy/test consumers; no credential was used.
- **Dependencies/CI:** reviewed Cargo features, lockfile, vendor rationale,
  supply-chain gate, release workflow permissions and pinned ecosystem fixtures.
  Retained existing dependencies without proof that removal preserves consumers.
  The performance workflow's vcpkg setup remains because it builds arbitrary
  historical revisions, whose OpenSSL feature selection is not guaranteed to
  match this checkout. Current-source workflows use Cargo's vendored build;
  `openssl-sys` ignores `OPENSSL_DIR` on that path unless `OPENSSL_NO_VENDOR`
  explicitly disables it. The inspected workflows do not disable vendoring.
- **Dead weight/docs:** TODOs predominantly describe unfinished profiler
  instrumentation. Their presence does not prove dead supported code. Earlier
  historical audit records and platform fixtures are retained as evidence.

## Security

Sealed baseline scan: `9afb14d3-4da3-43fd-a91c-b35a31c054db`, one low-severity
confinement finding. Directory creation is the demonstrated impact; existing
checks stop outside payload publication. An attacker needs an archive-consuming
workflow and an existing outside-pointing alias. Concurrent hostile mutation of
the extraction tree is explicitly outside the existing contract. In-root aliases
and final-target symlink refusal must remain compatible.

Baseline report:
`/Users/robertdevore/.codex/state/plugins/codex-security/scans/kujo/27be6e55247e071509f8791799e448ed55659dc6_20261019T132821Z_7pz57wib/report.md`.
The scan preserves starting-source evidence; this engineering report records the
later changes. Its measured rollout usage was 6,088,378 total tokens, 6,065,534
input tokens including 5,840,640 cached input, and 22,844 output across four
threads. These are scan-tool accounting, not runtime token savings.

Upstream ZIP advisories concern extraction APIs. Kujo uses its own extraction
loop and bounded upgrade reader, so version matches are not proof of those exact
upstream exploits in Kujo. Sources:
[RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
[0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html),
[0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html).

ZIP 9.0.5 was evaluated locally, including its Rust 1.88 floor and changed
`data_start()` return type. The migration was not retained: public
`Value::ZipArchive` exposes `ZipWriter<File>` (`src/interpreter/value.rs:1335`), so
silently changing the ZIP major version can break Rust embedders constructing that
variant with ZIP 7. No evidence establishes that those consumers do not exist.
Cargo.toml, Cargo.lock and upgrade.rs are unchanged. A deliberate API migration or
compatible upstream backport is needed; no advisory suppression was introduced.
The exploratory ZIP 9 audit is separate from the final unchanged-lock audit.

## Compatibility

Public Rust/Kujo API signatures, CLI commands/envelopes/exit codes, file formats,
schemas, configuration, environment names and dependency versions are unchanged.
Intentional corrections reject outside ZIP ancestor effects before mutation and
reload source changed during initialization. First-occurrence path precedence,
explicit discovery refresh, unchanged-source reuse and in-root aliases remain
covered. Metadata-preserving edits and transitive dependency invalidation remain
outside the existing freshness guarantee. No external consumer migration is
required for these retained changes.

## Cross-repository follow-ups

No required sibling implementation change identified. Updating this checkout does
not patch previously published runtime artifacts; release/backport decisions are
separate. External Workcell isolation and hosted publication approvals remain
outside this implementation's ownership.

## Remaining work

- **P0/P1:** no additional validated finding deferred from this pass.
- **P2:** YAML parser and Linux keyring dependency migrations need compatibility
  fixtures and platform verification; dependency warnings remain release blockers.
- **P2:** choose a compatibility-preserving ZIP remediation/release strategy;
  three version-based advisory matches remain, without a demonstrated call to
  their affected upstream extraction functions from Kujo.
- **Needs more evidence:** whole-repository exhaustive security coverage, native
  Windows execution, external DB/provider integration, content/dependency-based
  module freshness, and consistent imported-source resource limits.
- **Not worth changing:** cosmetic refactors, removing compatibility APIs without
  consumer evidence, speculative cache layers and timing-sensitive CI budgets.

## Verification receipt

Build/test environment: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1`, single Cargo job. This
reduces local disk use without changing test assertions or feature selection.

| Command | Result |
| --- | --- |
| `cargo test --locked --no-fail-fast -j 1` (baseline) | Exit 101: only VM generated-inventory freshness failed (nine days versus seven-day budget); library 966 and binary 1,009 tests passed |
| New archive/cache regression binaries against the baseline library | Expected failures: both outside-directory creation cases and stale self-replacement exports; compatibility controls passed |
| `cargo test --locked -j 1 --test archive_ancestor_confinement --test module_cache_source_state --test native_api_security_boundaries --test import_capability_policy` | Passed: 3 archive, 3 cache, 94 native-boundary and 6 import-policy tests |
| `cargo test --locked --no-fail-fast -j 1` (final) | Passed: 3,104 test results across 118 targets, zero failures, 20 existing ignores (including doc tests); includes docs/examples, CLI/JSON, diagnostics, JIT and generated contracts |
| `cargo clippy --locked --all-targets --all-features -j 1 -- -D warnings` | Passed |
| `cargo check --locked --no-default-features -j 1` | Passed; emits 17 feature-disabled unused/dead-code warnings outside the retained changes |
| `cargo fmt --check` | Passed before and after changes |
| `target/debug/kujo test --runtime vm` | Passed 150/150 runnable fixtures; 11 policy skips |
| `target/debug/kujo test --runtime dual` | Passed 150/150 runnable fixtures; zero interpreter fallbacks; 11 policy skips |
| `bash scripts/generate_unsafe_inventory.sh --strict` | Passed; date and shifted source anchors only |
| `bash scripts/generate_vm_runtime_mismatch_inventory.sh --runner target/debug/kujo --strict` | Passed; date only |
| `bash scripts/repo_hygiene_audit.sh` | Passed before and after |
| `bash .github/scripts/check-release-state.sh` | Passed before and after |
| `bash .github/scripts/check-contract-version-sync.sh` | Passed before and after |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` | Baseline and final fail: 3 ZIP advisory matches and 4 maintenance warnings; no lockfile change |
| Ruby `YAML.load_file` on all three changed workflows | Passed |
| `git diff --check` | Passed |
| `node .audit-evidence/hardening-2026-10-19/paired-benchmark.cjs` | Passed: six alternating baseline/candidate invocations, all export assertions; raw samples committed |

Full logs and the exploratory dependency evaluation remain in
`.audit-evidence/hardening-2026-10-19/`; the committed benchmark JSON preserves
the final raw timing samples. Native Windows/macOS-arm64/Linux matrices and live
external database/provider tests were not executed on this macOS x86_64 host.
The baseline build and an accidental default-profile generator build were
interrupted for disk management; the listed completed checks use the explicit
bounded-build settings and existing runner above.

## Durable unresolved findings

SignalBox project `kujo`, exact-ID and concept retrieval verified:

| Topic | Capture | Signal |
| --- | --- | --- |
| ZIP advisory remediation and public Rust API compatibility | `cap_ec1b8c3d-b414-43b4-ad38-241c6553c5cc` | `sig_1fcdc4fd-5946-40cb-a30b-e20a9d90e9a4` |
| Linux keyring transitive maintenance migration | `cap_6c5d0160-15ed-4272-b354-9bf01bdf6538` | `sig_14dd7466-89a9-4432-b5bc-1996b9e0cdd2` |

Deduplication skipped existing YAML maintenance capture
`cap_2624e9fd-834c-4509-8560-652c676e89e3` / signal
`sig_6660250b-0fbd-4535-a22c-42f56165612f`, and general module-freshness capture
`cap_2a7f3277-def7-47c4-ab41-e7ce1fbc4a35`. Completed fixes, routine verification,
and speculative imported-source limits were rejected as new SignalBox entries.
No downstream tasks, dispositions, or handoffs were created there.
