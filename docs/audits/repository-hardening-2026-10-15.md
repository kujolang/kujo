# Repository hardening — 2026-10-15

## Repository and review boundary

- Repository: `kujolang/kujo`; branch: `main`.
- Starting SHA: `d6ed9c0cfa76a1f17fb87020b9e9a77950c7fc25`; initially clean.
- Ending implementation SHA: `996eb92c348d5a2f43941d5c3411848f0c7eba8a`.
- Purpose: Rust-hosted language, default bytecode VM, interpreter, capability-aware
  native APIs, deterministic CLI/package contracts, AI helpers and local tooling.
- Integrations: reqwest/HTTP, OpenSSL/rustls, SQLite/Postgres/MySQL, image/PDF,
  Cranelift, local package locks and external Kennel consumers; GitHub/npm release
  pipelines. Stable version remains 1.8.1; no new release is implied.

This pass examines current implementation rather than repeating the earlier
hardening reports. Review includes module resolution/evaluation/cache authority,
VM/interpreter native dispatch and continuation policy, network destination and
redirect controls, AI replay and credential redaction, HTTP stream collection,
static file selection, archive/decompression limits, subprocess ownership,
package serialization, token-context selection, CLI source parsing, test gates,
release workflows, and locked dependency advisories. Independent architecture,
baseline security and focused import reviews supplemented parent source tracing.
This is risk-oriented coverage, not a claim of exhaustive line-by-line review of
all source, examples, vendored dependencies and platform implementations.

## Baseline

The first `cargo fmt --check` passed on the unchanged checkout. Full baseline
`cargo test --locked` was interrupted before completing compilation because the
host filesystem approached exhaustion. A subsequent low-debug, nonincremental
full attempt failed with `rustc-LLVM ERROR: IO failure on output stream: No space
left on device` while compiling `hyper`. This is an environment failure, not an
observed failing Kujo assertion. Repository-local `cargo clean --release` and
`cargo clean -p kujolang` reclaimed generated artifacts; no source or unrelated
project data was removed.

The complete baseline subsequently ran with low-debug, nonincremental builds and
socket fixtures enabled. Library tests passed 964/964 (8 ignored), binary tests
passed 1,007/1,007 (8 ignored). The full command failed six targets: three targets containing newly added
regression tests and three existing defects. Existing defects were a hung
live-provider mock (its client had already exited with missing random authority),
a TODO inventory line drift (type_checker.rs 5165 → 5169), and a missing
http_stream_collect reference-table entry. The hung target was terminated after
its other tests completed, allowing --no-fail-fast to finish the remaining suite.
A corrected mock exposed the original command failure without changing runtime
source: uuid_v4 was denied in the live bridge. Separate schema probes failed the
byte, count and combinator budgets while the valid large-path control passed.
Baseline direct probes also confirmed file effects in test bodies/setup/teardown
and all three bytecode mapper aliases under --untrusted.

Detailed local logs: `/tmp/kujo-hardening-20261015/`. They contain build evidence,
not durable public artifacts; the final receipt below preserves conclusions.

## Findings

| ID | Priority | Area | Finding / evidence | Action | Status |
| --- | --- | --- | --- | --- | --- |
| H01 | P0 | Capability propagation | `ModuleLoader::load_module` creates a trusted interpreter for imported initialization; both runtime import paths omit caller policy | Propagate authority and invalidate evaluated cache on policy changes; test direct, nested and selective imports | Reproduced; fixed; focused regressions pass |
| H02 | P1 | AI egress | Initial AI allowlist check is followed by generic redirect transport; 307/308 can forward body outside the approved origin/path | Request-local AI redirect authorization with existing destination and hop limits | Reproduced; fixed; focused regressions pass |
| H03 | P2 | Static serving | Public-looking in-root symlinks can resolve to filtered private names; custom 404 follows the same pattern | Check canonical root-relative private components while preserving public aliases | Reproduced with in-root private aliases; fixed; focused regressions pass |
| H04 | P1 | Dependencies | Locked rustls 0.23.45 matches four October 14 advisories; patched range starts at 0.23.46 | Minimal locked patch update | Updated to 0.23.46; no advisory matches remain |
| H05 | P2 | Dependencies | `unsafe-libyaml` is unmaintained and `cargo audit --deny warnings` rejects it | Assess a replacement against YAML compatibility before migration | Open |
| H06 | P2 | Documentation | `--deny-private-net` described as independent of environment, but the explicit private-network override remains honored | Correct documentation to actual established precedence | Fixed |
| H07 | Needs more evidence | Module freshness | Cache uses modification time and length, not content or dependency fingerprints; metadata is sampled again after evaluation | Define desired hot-reload contract before changing evaluation/cache semantics | Open |
| H08 | Needs more evidence | Module resource bounds | Imports read/tokenize complete source without the CLI's 1 MiB source check | Shared bounded source-read design should preserve CLI error contracts | Open |
| H09 | P0 | Test authority | TestRunner created trusted child interpreters; setup structured errors and teardown failures could be ignored | Inherit policy and fail on hook errors | Reproduced; fixed; focused regressions pass |
| H10 | P0 | Parallel authority | Bytecode mapper workers created trusted VMs; all three aliases reproduced denied file writes | Copy caller policy into each worker VM | Reproduced; fixed; focused regressions pass |
| H11 | P1 | Schema resources | Repeated required-property errors clone long instance paths outside the node budget | Bound shared diagnostic count and bytes before allocation | Three baseline regressions reproduced; fixed; focused regressions pass |
| H12 | P2 | Cached imports | get_all_exports cloned exports again after owning a cloned Module | Move the owned exports; paired microbenchmark | Verified; paired timing results below |
| H13 | P1/P2 | Existing verification failures | Live agent bridge omitted randomness, mock could hang, TODO inventory and reference table drifted | Narrow bridge grant, explicit mock shutdown, generator refresh and missing reference row | Baseline reproduced; fixed; focused regressions pass |

## Changes implemented

- **Execution authority:** module loaders retain their evaluator policy and clear
  evaluated exports when that policy changes. Interpreter construction/setters
  synchronize the loader. Test bodies and hooks, plus bytecode mapper workers,
  inherit their parent policy. Direct/nested/selective imports, policy transitions,
  all mapper aliases, denied effects and permitted controls have regressions in
  tests/import_capability_policy.rs. No filesystem capability is added merely to
  import pure source. Previously obtained values are not retroactively revoked.
- **AI egress:** an AI-specific client snapshots the allowlist for live transport
  and checks each redirect while retaining strict DNS/private-literal enforcement,
  no-proxy behavior in strict mode and reqwest's hop limit. Redirect denials retain
  endpoint_denied in both legacy and structured errors. The common transport serves
  all high-level AI helpers; strict replay still exits before client construction.
  Loopback tests cover VM/interpreter, 307/308, same/different origin, allowed and
  denied paths, and unset-allowlist compatibility. Generic HTTP is unchanged.
- **Schema diagnostics:** shared counters permit 10,000 errors and 4 MiB of
  cumulative diagnostic string bytes. Required/additional-property paths and type
  messages reserve bytes before formatting. Checked arithmetic and branch-shared
  accounting prevent overflow or resetting the budget through combinators.
  Exhaustion is an operational error, never silent truncation. Tests cover exact
  boundaries, Unicode pointer escaping, overflow, amplification and valid paths.
- **Static files:** after canonical containment, root-relative private components
  are rejected for normal files and custom 404 pages. Public aliases still work.
  This strengthens existing lexical filtering without claiming race-proof serving
  from an attacker-writable directory.
- **Dependencies and baseline repairs:** Cargo.lock changes only rustls version
  and checksum. The live AI SDK bridge grants random for UUID creation; fixture
  execution keeps its existing authority. The provider mock is explicitly unblocked
  after client completion, so failures are reported rather than hanging. The
  standard-library reference includes http_stream_collect. The existing TODO
  generator refreshed only its date and the stale source location. The unsafe
  inventory generator refreshed its date and one moved non-executable string
  location; counts and classifications remain unchanged.
- **Import efficiency:** return the already-owned export map instead of cloning it
  again. No new cache or freshness semantics are introduced. The local benchmark
  checks export counts and retains raw samples; there is no noisy timing gate.

Affected contracts are documented in CHANGELOG.md, AI_RUNTIME.md,
NATIVE_API_SECURITY_POSTURE.md, STANDARD_LIBRARY.md, STANDARD_LIBRARY_REFERENCE.md
and AGENT_PROJECT_IMPLEMENTATION_REPORT.md. Existing full-suite discovery picks up
all regression tests; no new CI timing threshold or disabled assertion is needed.

## Performance and efficiency

The owned-export change removes one redundant map clone per get_all_exports call.
The committed scripts/module_export_bench.rs was compiled against the original
and candidate libkujo with identical unoptimized rustc flags. Three paired runs
alternated order (before/after, after/before, before/after). Each case warmed two
batches, then measured seven batches of 100 cached imports and checked export
counts. The table reports the median of the three run medians on macOS x64.

| Exports | Before (ms / 100 cached imports) | After (ms / 100 cached imports) | Change |
| ---: | ---: | ---: | ---: |
| 100 | 34.50 | 27.26 | -21.0% |
| 1000 | 160.13 | 86.61 | -45.9% |
| 5000 | 559.03 | 341.50 | -38.9% |

These are local microbenchmark observations on a shared host, not release-build
or application-wide speed guarantees. Initial measurements taken during baseline
compilation were substantially noisier and are excluded from the comparison.
The [raw paired samples](repository-hardening-2026-10-15-benchmark.json) are committed
with compiler/profile metadata.
No RSS, build-size or provider-token reduction is claimed. The token estimator/context fitter already borrows message data,
validates all messages, estimates once and performs a linear selection pass;
its bounds and deterministic estimator contract should be retained. HTTP stream
collection has a bounded event channel and response ceiling, and intentionally
retains the final body for compatibility; it is not constant-memory streaming.
Existing release-gate output logging already preserves full evidence with concise
receipts when `KUJO_RELEASE_GATE_LOG_DIR` is set. No redundant output framework,
cache, dependencies, or speculative timing ratchets are warranted.

## Security

Baseline security scan `702c7acc-08d2-48a3-a00d-80f649234f21` is complete. Its
five findings describe the starting SHA; their remediation is documented here.
The local scan report is at
`/Users/robertdevore/.codex/state/plugins/codex-security/scans/kujo/d6ed9c0cfa76a1f17fb87020b9e9a77950c7fc25_20261015T051852Z_yees1row/report.md`.

Confirmed import defect violates the documented capability boundary even though
Kujo is explicitly not an OS sandbox. A sibling local module suffices; arbitrary
path traversal and remote loading are unnecessary. Cache and nested initialization
must be covered by the same correction.

AI redirect exposure requires an initially approved server with malicious or
attacker-influenced redirect behavior. The finding concerns request-body egress;
no claim is made that reqwest forwards Authorization across origins. Strict DNS,
private-address checks, timeouts, response-size limits and network-free strict
replay remain relevant countercontrols.

Static private-file aliases require an existing alias inside the served tree.
Concurrent hostile parent-directory replacement requires an independently writable
root and is not equated to a remote-only traversal vulnerability. Archive entry
symlinks/traversal and decoded-size limits are enforced; descriptor-relative
isolation of a hostile mutable extraction tree is a separate architectural issue.
Process-group escape by deliberately daemonizing Unix descendants is not a new
sandbox guarantee; Windows job ownership and fail-closed admission were reviewed.
No verified embedded credential exposure was admitted.

Dependency evidence uses advisory database commit
`edb58f5974b83eb8302332821cb6d389777aa351` (1,329 advisories; 652 locked packages).
The four rustls advisories are
[RUSTSEC-2026-0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html),
[RUSTSEC-2026-0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html),
[RUSTSEC-2026-0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html), and
[RUSTSEC-2026-0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html).
These are dependency version matches, not four reproduced Kujo exploits:
custom session Debug logging, ECH early data, optional Brotli decompression, and
key-share prediction each have specific prerequisites. No new advisory exception
is authorized by this audit; the existing build-only bincode exception remains
separate from the new YAML maintenance warning.

## Compatibility

Public function signatures, CLI options and exit codes, JSON envelopes, configuration
formats, environment names, cassette hashes and package lock schemas are preserved.
Intentional behavior changes deny previously unauthorized child effects/redirects,
reject private static aliases, fail test hooks that fail, and return an operational
error for excessive schema diagnostics. External callers that depended on those
incorrect behaviors are affected. The live agent bridge explicitly gains random
for provider-client UUIDs; deterministic fixture mode does not. Valid pure imports
remain available without general filesystem-read permission. Schema limits are
not a whole-validator heap bound.

## Cross-repository follow-ups

No sibling repository changes are required for the identified runtime corrections.
Published versions and transport backport branches are outside this checkout's
implementation scope; their exposure/remediation requires separately verified
release/backport decisions. Do not infer that changing main patches shipped
artifacts.

## Remaining work

- **P0/P1:** no unresolved validated code finding from this pass; no claim of a
  published patched release. Full-suite execution and environment-related retries
  are recorded below.
- **P2:** H05 safe-Rust YAML parser migration needs compatibility fixtures.
- **Needs more evidence:** H08 shared import-source limits need a public contract
  decision; the existing 1 MiB limit is specifically documented for CLI entrypoints.
- **Needs more evidence:** H07 hot-reload freshness semantics; hostile mutable-root
  deployment requirements; native Windows behavior requires platform CI.
- **Not worth changing:** existing bounded token selection, output receipts and
  process controls without new counterexamples; profiler TODOs are explicit future
  instrumentation rather than proof of removable dead code. No cosmetic rewrite.

## Verification receipt

| Command | Result |
| --- | --- |
| `cargo fmt --check` on starting tree | Passed |
| `cargo test --locked` | Interrupted during dependency acquisition to avoid disk exhaustion |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast` | Failed before tests: no space left on device |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` | Failed: four rustls advisory matches and unsafe-libyaml maintenance warning |
| Full baseline: same environment, cargo test --locked --no-fail-fast -j 1 | Completed; six affected targets as detailed above; existing library/binary suites passed |
| Baseline schema regression binary | 3 expected failures, valid-path control passed |
| Corrected mock against original runtime | Exposed existing missing-random capability error; no hang |
| CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked --lib -j 1 | Passed |
| cargo fmt --check after source changes | Passed |
| Focused post-fix library/import/schema/static/agent suites | 966 library tests, 6 authority tests, 4 schema tests, 32 serve tests and 11 agent tests passed; AI fixture initially used an invalid JSON builtin and was corrected |
| cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json after rustls update | Zero vulnerabilities; still fails solely on unsafe-libyaml maintenance warning |
| cargo machete | Unavailable: subcommand is not installed; no dependency deletion based on guessed usage |
| bash scripts/generate_v1_code_todo_triage.sh --strict | Passed; inspected date and one source-line update |
| Corrected redirect/schema regression targets | Passed: 1 redirect matrix test and 5 schema tests |
| CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --locked --all-targets --all-features -j 2 -- -D warnings | Passed |
| target/debug/kujo test --runtime vm | Passed 150/150 runnable fixtures; 11 policy skips |
| target/debug/kujo test --runtime dual | Passed 150/150 runnable fixtures; 11 policy skips; no interpreter fallback |
| CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked --no-default-features -j 1 | Passed; 17 existing dead-code warnings in feature-disabled code |
| Full post-fix build with -j 2 | Failed on parser_precedence without a detailed diagnostic; no source change was made |
| Isolated parser_precedence retry with -j 1 | Passed all 12 tests |
| Full post-fix suite with -j 1 | All targets passed except unsafe inventory location metadata; inspected and regenerated |
| generated_artifact_freshness_contract --exact generated_unsafe_inventory_artifact_is_fresh_and_matches_generator_output | Passed after generator refresh |
| Final full suite, same environment and -j 1 | All targets passed except generated artifact freshness and HTTP route concurrency, both reporting explicit No space left on device errors |
| CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked -j1 --test generated_artifact_freshness_contract --test http_route_concurrency | Passed all 8 tests on retry; completes passing coverage of every full-suite target without source changes |
| Paired export benchmark | Three alternating paired runs; export-count assertions passed; raw results linked above |
