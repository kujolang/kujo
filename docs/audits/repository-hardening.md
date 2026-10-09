# Repository hardening audit — 2026-10-09

## Repository and scope

- Repository: `kujolang/kujo`; branch: `main`.
- Starting SHA: `853ad9c61ab7ba09bfa58f65c6d292cd622644a7`, initially clean.
- Ending implementation SHA: `5681dd8618859d79fef7e62232517651930c5def`.
  The subsequent documentation/publication commit contains this report; its SHA is
  available with `git log -1 --format=%H -- docs/audits/repository-hardening.md`.
- Runtime: Rust implementation of Kujo 1.8.0: lexer/parser → AST → compiler → VM;
  interpreter fallback, CLI, LSP, native effects, package manifests and AI primitives.
- Users: local script authors, automation/agents, ecosystem package authors, Rust
  embedders and CLI consumers. Trusted execution intentionally has host privileges.
- Integration contracts: CLI JSON/exit codes, VM/interpreter parity, standard
  library names/arguments, manifests/lockfiles, AI replay files, capability flags,
  HTTP/process/filesystem semantics. Kennel registry and MCP policy live elsewhere.
- Dependencies: 652 locked packages; reqwest/Tokio, rustls, zip, serde, tempfile,
  cap-std and optional SQL/image/PDF/JIT feature groups. No dependency changes.

This is a broad engineering audit with focused source and regression review,
not a claim that every line of the 1,937-file repository received a security
review. Generated reports, vendor implementation internals, platform-specific
release binaries, remote database servers and live AI providers were not exhaustively
reviewed or exercised. All writes stay in this repository except requested local
memory/security records. No sibling repository changes; repository commits/push are explicitly authorized.

## Baseline

Host: macOS x86_64; Rust/Cargo 1.96.0. Debug info disabled for local Cargo checks
and tests to control build disk usage; default runtime features remain enabled.
Before production edits, formatting, locked build check, full socket-enabled
Cargo tests, all-target/all-feature Clippy, hygiene, release-state checks and the
existing dependency-audit policy passed. Full verbose logs are retained locally in
`.audit-evidence/hardening-2026-10-09/` (ignored); compact measurements are committed
under `docs/audits/evidence/`.

Pre-existing warnings: vendored tiny_http's unused import and dead marker trait.
Clippy already has broad library-level allowances: passing the existing gate
must not be read as enabling every library lint. Existing ignored tests remain
ignored; no assertions or timeouts were weakened.

Setup attempts: interrupted initial build; offline retry lacked two crates;
`cargo fetch --locked` succeeded. A redundant unsafe-gate build was canceled while
waiting for the Cargo lock; its generated timestamp-only inventory change was
restored. Full Cargo tests cover its contract tests. A first isolated benchmark
link failed on LLVM bitcode incompatibility; matching thin-LTO linking succeeded.
These are setup limitations, not baseline runtime failures.

Baseline reproductions on the starting-tree debug binary:

- Restricted `os_environ` with only filesystem read printed the injected synthetic
  environment value (exit 0), despite no environment-read grant.
- `shared_add_int(MAX_I64, 1)` produced an internal VM panic (exit 6).
- A ZIP declaring a one-byte entry decoded/extracted 4,096 bytes (exit 0).
- `spawn_process(["sh", "-c", "sleep 2 & exit 0"], {"timeout_ms":100})`
  took 2.15 s and returned `timed_out=false` because the descendant retained pipes.

## Findings

| ID | Priority | Area | Finding | Evidence | Action | Status |
|---|---|---|---|---|---|---|
| H01 | P0 | Network boundary | Private-destination checks missed redirects, second DNS resolutions and mapped IPv4 | `network_policy.rs`, HTTP/TCP/UDP callers; independent source review | Validate connector addresses and redirect literals; normalize mapped IPs | Fixed; regressions added |
| H02 | P0 | Capabilities | Environment enumeration and compound effects were assigned incomplete capabilities | Synthetic environment reproduction; dispatcher/capability map and native implementations | Correct primary/secondary gates; migration docs | Fixed; both runtimes covered |
| H03 | P0 | Archive resource limits | Metadata-only decoded-size checks could be bypassed | Forged one-byte ZIP extracted 4,096 bytes | Bound decoder and validate staged entry before publication | Fixed; both runtimes covered |
| H04 | P1 | Shared state | Unchecked integer addition panicked/wrapped under store locks | Starting-tree VM exit 6; checked arithmetic elsewhere | Checked addition, preserve value and locks on error | Fixed; both runtimes covered |
| H05 | P1 | Cassette persistence | Same hash/PID writers shared a staging filename | `store_ai_cassette` source; concurrent publication regression | Unique private staging, atomic replace, RAII cleanup | Fixed |
| H06 | P1 | Context fitting | Repeated cloning/recounting and vector removal made pruning quadratic | Same-source isolated benchmark; estimator implementation | Estimate once, prune in one ordered pass, share strings | Fixed; exhaustive small-input equivalence and maximum-size regression |
| H07 | P1 | Process lifecycle | Deadline stopped after direct child exit, before output drain | 100 ms timeout returned after 2.15 s | Keep deadline/cancellation through output drain; Unix group cleanup | Fixed for ordinary Unix descendants; platform limits below |
| H08 | P2 | Agent/release docs | Agent guide advertised stale 1.7 while source/tag/docs identify 1.8 | `AGENTS.md`, Cargo manifest, release guard | Correct version references; extend existing release gate | Fixed |
| H09 | P2 | External process bounds | GIF capture/deadline and Windows descendant cleanup were unbounded/incomplete | Original converter and `terminate_process_tree`; follow-up below | Shared bounded converter; per-command Windows jobs; regression fixtures | Implemented in `050a0cd`; see follow-up verification |
| H10 | P1 | Dependency security | Three GitHub advisories affect locked Hickory resolver 0.26.1 | Dependabot alerts 7–9, vulnerable ranges and upstream release notes | Require and lock 0.26.3 security/regression fixes | Implemented in `8e41c54`; verification below |

P0 is remediation priority for boundary/data-integrity defects, not a CVSS severity
claim. Network exploitation requires an application granting network access and
accepting attacker-controlled endpoints/DNS; trusted arbitrary-code execution is
not itself a vulnerability. No deployed application compromise was demonstrated.

## Changes implemented and compatibility

### Network destinations (H01)

Root cause: preflight DNS results were discarded by ordinary connectors, redirects
used default clients without equivalent policy, and IPv6 classification omitted
mapped IPv4. `src/network_policy.rs` now validates final TCP/UDP address sets and
uses a checked reqwest DNS resolver for strict HTTP, including redirected hosts.
Redirect literals are checked separately because literal URLs bypass DNS. All
ordinary native HTTP callers now supply their initial URL. UDP sends the validated
`SocketAddr`; explicit pinned/no-redirect clients retain their contract.

Affected callers: `src/builtins.rs`, native `http.rs`, `async_ops.rs`, `network.rs`.
Tests cover mapped-address classes, initial literals, mixed DNS answers, final
socket resolution, sync/async private redirects, public redirects and the default
loop limit. Public Rust builder signatures remain available; native dispatch uses
the URL-aware variants. Global strict clients now disable proxies; permissive mode
and the explicit global private-network override retain their behavior.

### Capabilities and ZIP extraction (H02/H03)

`capabilities.rs` moves `os_environ` to environment read. `copy_file`,
`io_copy_range`, `zip_add_file`, `zip_add_dir`, `unzip`,
`ssg_read_render_and_write_pages` and `gif_to_webp` require source read in addition
to write; GIF conversion also requires process execution. The dispatcher rejects
missing capabilities before native effects. Restricted callers need the missing
`--allow-env-read`, `--allow-fs-read` or `--allow-process-exec`; trusted mode is unchanged.

`filesystem.rs` limits actual decoding to declared size plus one byte (declared
sizes already have hard bounds), checks exact size and decoder errors, and atomically
publishes validated per-entry staging. Staged contents stay private until final
permissions are applied. Invalid files preserve existing targets and clean staging.
Read-only targets are rejected. Ordinary file modes are preserved; atomic replacement changes inode/hard-link
identity. This is not archive-wide rollback or protection against hostile concurrent
mutation of the extraction directory. `tests/native_api_security_boundaries.rs`
covers both runtimes, missing grants and forged ZIP contents.

### Shared state and cassette publication (H04/H05)

`concurrency.rs` performs checked signed addition before mutation; overflow returns
a catchable error without changing the value or poisoning shared-store mutexes.
`tests/concurrency_lifecycle_contracts.rs` covers both bounds, preserved values and
subsequent usable state in VM and interpreter.

`http.rs::store_ai_cassette` uses the existing tempfile dependency for unique private
same-directory staging and atomic replacement. Eight simultaneous same-key writers
must all succeed and leave one complete redacted cassette; failure cleanup preserves
the target. Cassette JSON and last-writer-wins semantics are unchanged. Unix cassette
permissions tighten to 0600. No crash-durability guarantee is added.

### Context fitting (H06)

`token.rs` shares immutable role/content/name strings, validates and totals all
message costs once, then drops eligible messages in order. System messages and
the final user message remain protected. Errors from oversized fields are still
reported even when a message would be removed. No tokenizer policy, result fields,
message normalization or estimate values change. Regression coverage compares a
recounting oracle over role combinations, model families, Unicode, optional names,
fixed/dynamic dictionaries and budgets; a 100,000-message case guards scalability
without a flaky wall-clock assertion.

### Process lifetime and agent guide (H07/H08)

`system.rs` continues cancellation/deadline checks until both output readers finish;
canceled results cannot report success. The Unix fixture's descendant is finite
for safe failure cleanup and tests timeout semantics in both runtimes, without a
machine-speed assertion. Windows and detached process-group behavior remain explicit
limitations. `AGENTS.md` now agrees with stable 1.8; the existing release guard checks
its stable-version line, so future releases cannot silently leave it stale.

Public API signatures, CLI command names/options, JSON schemas, file/config formats,
environment-variable names and exit-code mapping do not change. Intentional behavior
corrections are the extra capability denials, strict destination enforcement,
catchable shared overflow, invalid-ZIP rejection, private cassette permissions and
more complete timeout handling. Consumers relying on those bugs must migrate as
above. No ecosystem-wide rewrite or downstream release is required.

## Performance and efficiency

Focused optimized Rust probe, 2 warmups and 7 samples per size, fixed 64-token budget,
1 system message plus N identical user messages. Input construction is outside the
timed section. Both probes compiled the actual before/after token module with the
same existing release Value/library implementation, Rust toolchain, `-O`, thin LTO
and one codegen unit. This measures the helper, not end-to-end model/API latency.
Runs were sequential after baseline builds completed. Samples include ordinary host
noise; no production throughput or memory percentage is inferred.

| User messages | Before median | After median |
|---:|---:|---:|
| 100 | 2.745 ms | 0.150 ms |
| 1,000 | 271.315 ms | 1.567 ms |
| 5,000 | 11,314.066 ms | 16.350 ms |

Raw nanosecond samples: [before](evidence/token-context-before.txt),
[after](evidence/token-context-after.txt). Reproducible workload:
[`scripts/token_context_bench.rs`](../../scripts/token_context_bench.rs).
Compile against a compatible release `libkujo-*.rlib` with:

```sh
rustc --edition=2021 -O -C lto=thin -C codegen-units=1 \
  scripts/token_context_bench.rs --extern kujo=target/release/deps/libkujo-<hash>.rlib \
  -L dependency=target/release/deps -o /tmp/kujo-token-context-bench
/tmp/kujo-token-context-bench
```

Use the same library and compiler for comparisons; change only the included token
source. Actual measurements used `.audit-evidence/hardening-2026-10-09/token_probe.rs`
and `token_probe_candidate.rs`, linked to `libkujo-ccf49c1446c21feb.rlib` with these
flags; binaries were `token-baseline` and `token-candidate`.

Algorithmic evidence: quadratic repeated string counting becomes linear in total
text plus message count; full string clones and repeated message-vector clones are
removed. Memory/RSS, whole-build, binary-size and provider-billed token improvements
were not measured and are not claimed. Output estimates/retained context stay equal;
no necessary model context or observability is removed. Dependency count remains 652.

## Review coverage and decisions

- Frontend/CLI: inspected source limits, diagnostics/error dispatch and public JSON
  contracts; retained syntax, exit codes and bounded parser behavior.
- Runtime/state: reviewed task admission/cancellation, shared state, async ownership,
  VM/interpreter parity and module/analyzed-program caches. Existing channels/tasks
  have explicit limits; arbitrary shared-store eviction would break semantics.
- Resources/output: reviewed HTTP buffering, process output bounds, archive decoding,
  cassette writes and source/filesystem limits. Logs stay available; no production
  error suppression or truncation changes. Global state with explicit delete is not
  labeled a leak merely because it persists.
- Trust boundaries: native dispatch, network/DNS/redirects, file/archive paths,
  subprocess argv/environment, AI secrets/replay and host effects. Strict replay
  remains offline. Secret wrappers are redaction, not host-process isolation.
- Contracts/integrations: reviewed canonical docs, package lock/manifest workflow,
  generated artifacts, LSP and MCP delegation. MCP/provider policy belongs to its
  ecosystem packages; no speculative cross-repository interface changes.
- Dependencies: audit database commit `7eebec69c352c7191b1f13eb95dd510eeca5d1de`,
  1,296 advisories checked, zero vulnerabilities/warnings under existing policy.
  Existing `RUSTSEC-2025-0141` build-only bincode exception retained. Optional
  features and vendor patches have purposes; no unsupported dependency removals.
- Complexity/dead weight: removed repeated token accounting; retained established
  wrappers, public features, compatibility code and fixtures without proof of dead
  consumers. Broad VM/JIT refactors, stylistic churn and noisy new CI gates rejected.
- Regression ratchets: functional/equivalence/security regressions run in existing
  Cargo CI; existing release-state gate gains agent-guide coverage. No flaky timing
  threshold or machine-specific performance budget added.

## Remaining work and cross-repository follow-ups

- P0: no known introduced regression; no further validated P0 fix deferred.
- P1: processes deliberately escaping Unix groups still require external isolation;
  current code does not promise a sandbox. Windows/GIF H09 was implemented in the
  follow-up below, with platform verification recorded separately.
- P2: no remaining GIF capture/deadline implementation work. Converter heap use and
  output-file size are outside these process capture limits.
- Needs more evidence: RSS and end-to-end agent workloads; optional remote database
  behavior; release-platform integration; whole-repository exhaustive source coverage.
- P3 / not worth changing: cosmetic rewrites, redundant report generation, removing
  public features/dependencies without consumer evidence, speculative caching.
- Cross-repository: no required changes. Restricted consumers using the corrected
  builtins should review grants during adoption; exact downstream use was not proven.
  No sibling was modified or assumed private merely because it is internal-looking.

## Verification receipt

Implementation verification caught two defects in newly authored tests: a cassette
helper was initially called by the wrong name (compile failure), and the context
oracle generated budget -1 for an empty input (assertion failure). Both test defects
were corrected; existing validation and negative-budget coverage were retained.
The standard-library contract gate caught missing secondary capabilities in seven
inventory rows; the rows were corrected to match the runtime policy. The generated
unsafe inventory also correctly failed after source line shifts;
`bash scripts/generate_unsafe_inventory.sh --strict` refreshed it. Review caught
and corrected a staging permission race before publication: a separate empty mode
probe avoids writing payload bytes to any formerly public inode.

| Exact command | Before | After |
|---|---|---|
| `cargo fmt --check` | Pass | Pass |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked` | Pass | Pass on final sources |
| `CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked` | Pass | Pass |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --all-targets --all-features -- -D warnings` | Pass | Pass |
| `CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --test native_api_security_boundaries --test concurrency_lifecycle_contracts` | Included in full suite | Pass: 94 + 17 tests |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked --no-default-features --lib` | Not separately run | Pass; 17 optional-feature warnings, no errors |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo run --locked -- test --runtime vm` | Rust/CLI contracts cover baseline; not separately run | Pass: 150/150, 11 fixture-policy skips |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo run --locked -- test --runtime dual` | Rust/CLI contracts cover baseline; not separately run | Pass: 150/150, zero fallback, 11 fixture-policy skips |
| `bash .github/scripts/check-release-state.sh` | Pass | Pass |
| `bash scripts/repo_hygiene_audit.sh` | Pass | Pass |
| `cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` | Pass | Pass |
| `bash scripts/generate_unsafe_inventory.sh --strict` | Canceled redundant parent gate after generation | Pass; reviewed one shifted source anchor and generation date |
| `git diff --check` | Clean tree | Pass |

The full Cargo gate includes docs/examples, README, CLI/JSON, diagnostic goldens,
security, generated-artifact freshness, unsafe inventory and VM/interpreter parity
contracts. The base unsafe gate's two tests are therefore covered without a
redundant third build. Existing ignored cases comprise seven JIT microbenchmarks
in each test binary, two Dispatch-produced cross-component artifact fixtures, and
one environment doc test. No ignore policy was changed. Optional Miri, fuzz
campaigns, external databases/providers and other operating systems were not run.

Security tooling sealed scan `87dc5222-696b-4a31-b099-cefa3351ace7` against the
starting SHA with five medium-severity baseline findings and explicitly partial
source coverage. The working-tree fixes and final verification are recorded here;
the sealed scan intentionally retains the original snapshot. Its reported rollout
usage is 22,185,269 total tokens (22,114,650 input, including 21,477,760 cached input;
70,619 output) across three threads. This is audit-tool accounting, not runtime
model token consumption or a token-efficiency improvement claim.


## H09 follow-up — Windows descendants and GIF conversion

Requested explicitly after the original audit. Starting branch `main`, clean SHA
`4a2663a77690315761ec55712fb9cf43136aa20c`; process implementation SHA
`050a0cdda759e57ddab7606359497241dddf4fd5` (initial implementation `9fa3a88`).
The DNS dependency follow-up is `8e41c54`; the subsequent documentation commit
records the final receipt; obtain its SHA with the report's `git log` command above.
No sibling implementation changes or new packages. The existing windows-sys
dependency enables its ToolHelp feature; three Hickory crate versions change below.

### Changes and compatibility

- **Windows ownership:** `src/interpreter/native_functions/process_job_windows.rs` now creates a per-command,
  non-inheritable kill-on-close job, starts the command suspended, assigns it,
  and resumes its initial thread only after admission. Handle ownership uses
  `OwnedHandle`; failed admission/resume kills and reaps the suspended child.
  The shared native runner in `system.rs` terminates the whole job on timeout or
  cancellation even after the original parent exits. Closing the job on return
  also cleans up surviving background descendants. Existing runtime-wide jobs
  nest with command jobs; incompatible job restrictions fail closed.
- **GIF bounds:** `filesystem.rs` delegates to the same runner, preserving the
  boolean success value, argv execution, EOF stdin, and exact missing-tool and
  ordinary failure messages. Defaults are 30,000 ms and 1,048,576 captured bytes
  per stream. Failed stderr explicitly marks truncation; excess incidental output
  does not turn a successful conversion into a failure. Timeout and cancellation
  are errors rather than indefinite waits. No public arguments, ProcessResult
  keys, CLI flags, file formats, schemas, config or environment variables changed.
- **Real converter correctness:** the installed gif2webp 1.6.0 rejected the existing
  `-lossless` flag. Omitting it selects the converter's documented lossless default.
  An explicit final `--` input separator prevents filenames beginning with `-`
  from becoming options. A generated 2×2 GIF with such a name now converts with
  identical decoded RGBA pixels through both VM and interpreter. See the
  [gif2webp option reference](https://developers.google.com/speed/webp/docs/gif2webp).
- **Verification isolation:** the first full rerun failed the filesystem contract
  at `write_file_atomic_beneath`; it passed alone. Code-supported cause: the existing
  env/OS/path contract changes process-wide cwd while other tests derive temporary
  paths from cwd, then deletes that directory. Run the cwd-changing
  contract in its own test subprocess, preserving every assertion and eliminating
  its effect on concurrent tests. No production filesystem behavior changed.

The first native Windows CI compile caught a binary-only module reference that
an isolated helper cross-check could not detect. The helper now lives under the
shared native-functions module, so library, CLI and kujo-run resolve the same code.
CI logs also confirmed Cargo already compiles vendored OpenSSL (the manifest enables
that feature unconditionally). Removed the preceding vcpkg OpenSSL build, whose
copy was unused by openssl-sys's vendored path. The first run spent 7m 28s in that
redundant setup step; no end-to-end build speed percentage is claimed.

### Regression coverage and measured bounds

`system.rs` has hermetic converter fixtures for success, missing executable,
ordinary/empty failure, excess output, deadline and cancellation. A failed fixture
writes 65,536 bytes to each stream; with a 1,024-byte test capture limit the returned
stderr contains exactly 1,024 payload bytes plus an explicit truncation marker.
A public-CLI fixture emits exactly 2,097,152 stderr bytes; each runtime reports
exactly 1,048,576 payload bytes plus the truncation marker and diagnostic framing.
This fixture sets PATH only on the Kujo subprocess and requires no installed codec.
A 100 ms deadline stops a finite 3-second fixture with a less-than-2-second outer
assertion. These demonstrate bounds, not a general throughput or RSS improvement.
Previously the converter had neither deadline nor capture limit.

`tests/process_lifetime_contracts.rs` adds Windows parent-exit/inherited-pipe tests
for timeout and cancellation in both runtimes, retaining actual descendant process
handles and asserting termination. Existing forced-runtime-exit/nested ownership
coverage remains. The filesystem conformance workflow runs these plus fake GIF
fixtures on Linux, macOS and Windows. Linux installs WebP and explicitly runs the
otherwise opt-in real-codec test; ordinary local suites do not require that tool.
Generated unsafe inventory was regenerated with zero unclassified entries.
The exact executable-site gate increases from 86 to 99 after review of twelve
new lifetime-module sites (checked handle adoption, sized buffers, suspended-child
assignment/resume and job termination) and one test wait on an owned process handle.
Module counts remain exact at 4 runtime-lifetime, 12 command-job and 4 lifetime-test
sites; other module
ratchets are unchanged. The initial full run correctly rejected the old budget;
all three inventory tests passed after this explicit reviewed baseline update.

### Remaining limits

Kujo remains a trusted-code runtime rather than a sandbox. A Unix descendant can
escape its process group. On Windows, stable Rust requires suspended spawn followed
by job assignment: forced runtime termination in that narrow interval may orphan a
suspended child, although application code has not run. The existing
`--kill-children-on-exit` outer job covers this interval as well. Whole-runtime
external supervision remains appropriate for hostile code. GIF wall-time/capture limits do
not cap converter heap usage or output-file size, and conversion failures retain
the existing converter-owned partial-output semantics. No cross-repository change
is required. Windows background descendants intentionally cannot outlive a native
command call; consumers relying on that behavior must use an external supervisor.

### Follow-up verification receipt

Local logs: `.audit-evidence/process-followup-*.log`, `gif-followup-tests.log`,
`gif-real-tests.log`, and `windows-process-check.log` (ignored, concise evidence here).

| Command/check | Result |
| --- | --- |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test process_lifetime_contracts` before edits | Passed on host; [baseline CI 37944723166](https://github.com/kujolang/kujo/actions/runs/37944723166) at starting SHA subsequently passed Windows, Linux and macOS |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked gif_converter --lib` | 4 passed |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test gif_conversion_contracts` | Public output-bound test passed in both runtimes; real-codec test intentionally opt-in |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test unsafe_inventory_contract` | 3 passed after reviewed Windows FFI baseline update |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test gif_conversion_contracts -- --ignored` | Passed, real WebP 1.6.0, both runtimes; initially exposed and then verified the lossless flag fix |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked` | Passed |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed; existing vendored tiny_http warnings only |
| `cargo fmt --check`; `git diff --check` | Passed |
| `bash scripts/generate_unsafe_inventory.sh --strict` | Passed |
| `cargo check --manifest-path .audit-evidence/windows-process-check/Cargo.toml --target x86_64-pc-windows-msvc` | Passed; source-including probe type-checks the actual Windows lifetime module, not a native execution claim |
| `CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast` | Passed on final `8e41c54` code/dependencies: 2,980 tests across 104 test/doctest receipts (947 library and 990 binary unit tests), zero failures; opt-in real codec tested separately. Intermediate runs exposed the cwd race, docs arity mismatch and reviewed FFI budget update; all are corrected. |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo run --locked -- test --runtime vm` and `--runtime dual` | Each passed 150/150; 11 policy skips, dual used zero interpreter fallbacks |
| Native platform CI | [Conformance run 37950125629](https://github.com/kujolang/kujo/actions/runs/37950125629) at `050a0cd` passed Linux, macOS and Windows, including 4 Windows GIF unit fixtures and 5 Windows lifecycle tests (10.24 s), with both runtime modes. The initial `9fa3a88` run caught the CLI-only module reference. Final code/dependency [conformance run 37950698454](https://github.com/kujolang/kujo/actions/runs/37950698454) at `8e41c54` also passed all three platforms; Windows lifecycle tests passed in 10.23 s. |


### Newly surfaced dependency advisory follow-up

The push surfaced Dependabot alerts 7–9 on `hickory-resolver` 0.26.1:
[DNSSEC validation failures](https://github.com/advisories/GHSA-5j98-2g5x-46v6)
and [unbounded truncated-response retry](https://github.com/advisories/GHSA-6w6g-hm98-mhgm)
are rated high; [irrelevant CNAME following](https://github.com/advisories/GHSA-6f2x-v7q7-m7m5)
is rated medium. These are upstream dependency findings, not newly claimed Kujo
exploit reproductions. The manifest enables the resolver in the default network
feature; the locked version matches all three advisory ranges.

Updated the manifest minimum and lockfile to 0.26.3 for resolver/net/proto, retaining
all other package versions and the package count. The upstream
[0.26.3 release](https://github.com/hickory-dns/hickory-dns/releases/tag/v0.26.3)
also corrects DNSSEC and other regressions in the first patched 0.26.2 release.
Resolver minimum Rust remains 1.88, below Kujo's 1.89 floor. No Kujo API/schema
changed; upstream malicious/invalid DNS failure behavior is intentionally corrected.
Six existing DNS/options/negative-answer/DNSSEC/private-destination contract tests
passed under 0.26.3. Full post-update tests and Clippy are recorded in the final receipt.
`cargo audit --deny warnings --ignore RUSTSEC-2025-0141 --json` passed with zero
vulnerabilities and no remaining policy warnings. After the update push, GitHub
reported zero open Dependabot alerts; the three vulnerable-version alerts closed. No new suppression was added; the existing
build-only unmaintained bincode exception remains the original policy.
