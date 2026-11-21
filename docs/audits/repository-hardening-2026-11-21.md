# Repository hardening on 2026-11-21

## Repository

The review started from clean `main` in `kujolang/kujo` at
`f4a9f7f961e4b4c4cc8fbd546170d112a90b6fc0`. Kujo 1.10.0 is a Rust-hosted
language runtime with VM and interpreter execution, CLI and LSP tools, local
package and workflow-pack discovery, and deterministic AI primitives. The main
integration boundaries considered here were OS process and filesystem APIs,
Serde and YAML manifests, Rayon vector scoring, the default Cargo feature
set, and the Kennel, MCP, Agents SDK, and AI SDK handoffs.

Ending source and test SHA is `4acd5d3e68d14c71ae942e5135776965cbe01f06`. The next
commit records this audit and its verification receipt.

## Baseline

The host ran macOS 26.6.2 x86_64 with Rust 1.96.0. The clean-source
`cargo fmt --check`, `cargo check --locked`, and `cargo audit --deny warnings`
passed. RustSec database `55e1c3dfbd834237a05bbbf7b50ba3b5be27b662` contained
1,555 advisories and the root lockfile contained 625 dependencies. The eight
standalone tests from the original workflow process runner also passed.
The full baseline `cargo test --locked` was stopped during dependency
compilation because local space was insufficient for the 280 Rust integration
files and their linker outputs. It produced no baseline test verdict. Reclaiming
old Cargo binaries and using debug-info-free test builds made a broad targeted
matrix practical.

## Findings

| ID | Priority | Area | Finding and evidence | Action | Status |
| --- | --- | --- | --- | --- | --- |
| H01 | P0 | Workflow subprocesses | The timeout path called `try_wait` before reading either pipe. A child producing more than the pipe buffer could make no progress. The no-timeout path held complete output in memory. The line-first formatter could exceed the byte budget. | Use the existing runtime capture and process ownership, enforce both output ceilings, preserve UTF-8, and report truncated Git status without an exact count. | Implemented |
| H02 | P0 | Unix process lifetime | The native deadline terminated the child group and then joined blocking readers. A writer outside the group could keep that join waiting. | Make Unix reads respond to the cancellation flag while retaining the existing process ownership and Windows job behavior. | Implemented |
| H03 | P0 | Agent Project credentials | Authorization updates loaded and rewrote the same file without mutual exclusion. PID-derived staging conflicted within a process and could leave debris. Windows removed the old file before promotion. | Serialize production read-modify-write operations using a persistent project lock and replace with a uniquely staged file. | Implemented |
| H04 | P1 | Agent Project diagnosis | The credential scan followed directory and file links, inferred safety from path metadata, and then read without an I/O bound. | Walk retained directories without following links, read only regular files, enforce the existing 1 MiB ceiling through the opened handle, and reuse one dependency snapshot per report. | Implemented |
| H05 | P0 | Pack discovery and registry | Directory iteration order controlled collision winners, and a rejected pack could change namespace metadata or publish some commands before a later collision. The fallback format also sliced UTF-8 at byte 500 unconditionally. | Sort candidates within each source, stage commands and profiles before publishing, and truncate fallback text at character boundaries. | Implemented |
| H06 | P1 | Vector scoring | `numeric_vector` formatted an indexed string for every valid cell and `vec_top_k` sorted every score for short result sets. | Construct indexed text only on errors and partition scores before sorting the selected rows. | Implemented |
| H07 | P2 | Documentation | The workflow guide claimed no built-in packs while `builtins::register_all` registers Doctor. | Align the guide with the registry. | Implemented |
| H08 | P2 | Test resources | Archive tests retained their input and partial extraction after completion. Completed native test fixtures from two runs occupied 72,912 KiB, and the combined run reported host storage exhaustion. | Own archive test workspaces with `TempDir` and keep them alive through assertions. | Implemented |

## Review coverage

The pass focused on representative execution and failure paths. The coverage below
records where the inquiry stopped and why.

| Phases | Implementation and decision |
| --- | --- |
| 0–1 | Read the agent guide, build features, release gates, historical audits, and VM/interpreter entry points. Establish a clean format, compile and dependency baseline and record the full-suite limitation. |
| 2–4 | Trace workflow command capture, native readers, Git probes, matrix validation and sorting. Inspect bounded file reads in the native filesystem and response limits in the HTTP server before reusing established mechanisms. |
| 5–6 | Trace AI message normalization, request hashing, cassette replay and tool-loop bounds in `http.rs`. Strict replay resolves before transport and tool loops cap steps at 16. `mcp_make.rs` delegates to the ecosystem generator, so tool/schema and handoff pruning belongs with the contract owner. The local change keeps small diagnostic previews, structured-report ceilings and explicit overflow diagnostics. No provider tokens or schema sizes were changed or measured. |
| 7–9 | Follow reader EOF and cancellation, pack registry publication, and credential set, remove and read independently. Compare the credential scan with the existing capability-directory filesystem pattern. Review current capability-policy and process-group boundaries. |
| 10 | Keep manifest and report formats, package lock format, vector order and native process fields compatible. Check the CLI error policy, package/lock verification, and Agent Project doctor payloads. |
| 11 | Inspect Cargo feature ownership, the duplicate dependency tree, existing vendored patches, and the RustSec audit gate. Removing duplicate transitive versions would require compatibility work across their owners. Enable the event interface of the already-locked Rustix Unix dependency. Locked package versions are unchanged. |
| 12–13 | Add bounded-output, Unicode, cancellation, pack collision and deterministic search, concurrent credential update, staging cleanup, and vector ordering regressions to existing Rust suites. Regenerate the unsafe inventory after source anchors move. The executable unsafe budget remains 275. No timing threshold is added. |
| 14–16 | Reconcile workflow and Agent Project documentation, the CLI contract and standard library. Check current release status and known compatibility obligations. Keep public JIT/profiler and package helpers with active callers or tracked release obligations. Their TODOs do not establish dead code. |

## Compatibility and security

Workflow pack manifests and Doctor JSON keys and types are stable. Large reports
now complete when within the existing output budget and identify the applicable
byte and line limits when rejected. The existing Git status warning uses
`changed paths (preview truncated)` for incomplete previews. Source precedence
remains built-in, project, user, then environment. Path order resolves same-source
collisions and a rejected pack contributes no entries.

Workflow execution uses the native process cancellation and ownership policy.
A safe Rustix call polls the borrowed Unix pipe handle. A writer outside the
command's Unix process group can remain alive, while timed-out capture
returns the bytes already collected. Windows uses the existing job object.
Kujo's trusted workflow execution and capability boundaries still apply.

Agent Project credentials retain the `.env.local` format and credential lookup
order. A writer now also needs access to the existing ignored `.kujo-agent`
artifact location for its persistent lock. The lock contains no credentials.
The read path checks file type and Unix permissions on the opened handle.
Windows replacement uses the existing Tempfile backend. This publication
syncs the temporary file before replacement and makes no power-loss durability
claim for the containing directory. The doctor scan still reports relative
paths and omits credential values.

Vector scores retain their original arithmetic, ascending row-index tie
break, and validation order, including validation when `k=0`. There are no
new public signatures, config keys, or environment variables. Rustix is now
a direct Unix dependency for its event feature, using the same locked version.

## Performance and efficiency

The two capture streams each retain at most the configured byte budget plus
three bytes of UTF-8 lookahead and an 8 KiB read buffer before preview rendering.
The 512 KiB-per-stream regression verifies a child can finish while both previews
remain bounded. Stream draining avoids a per-chunk copy when no secrets are
configured. No process RSS, application throughput, build size, or token claims
are made.

The isolated vector benchmark compiled the exact baseline and candidate
`vector.rs` modules with `rustc --edition=2021 -O`, linked both against the
same Kujo library for its real `Value` types and the same Rayon 1.12.0 build,
and ran with `RAYON_NUM_THREADS=2`. Each workload used 100,000 rows and requested
10 results. The input at zero-based row `r` and column `c` was
`((13*r + 41*c) % 71) - 35` and each query element was 1. The timing covered
validation, scoring, selection and result construction, excluding input creation
and result printing. Three alternating pairs of runs each discarded three warmups
and kept seven samples per invocation. All paired runs returned identical row
indices and raw score bits.

| Dimensions | Baseline invocation medians, ms | Candidate invocation medians, ms | Median paired-workload speedup |
| ---: | --- | --- | ---: |
| 2 | 119.36, 120.09, 115.80 | 50.75, 50.47, 50.71 | 2.35× |
| 32 | 1,159.29, 1,188.38, 1,357.64 | 207.51, 202.50, 248.16 | 5.73× |

The speedup is the ratio of the medians of the three invocation medians.
The output check covers every returned score in this workload. The numbers
describe small top-k selection over large matrices on this host.

## Cross-repository follow-ups

None required for these changes.

## Remaining verification limits

The full Cargo matrix exceeded available disk space. Verification ran on
macOS with default runtime features. Windows and Linux platform jobs, full
all-target Clippy, MSRV, and live provider release rehearsal were outside this
local verification.

## Verification receipt

- `cargo fmt --check`, `cargo check --locked`, `cargo clippy --locked --bin kujo -- -D warnings`,
  `cargo audit --deny warnings`, `bash scripts/repo_hygiene_audit.sh`,
  `bash scripts/generate_unsafe_inventory.sh --strict`, and `git diff --check`
  passed. Cargo still reports the existing dependency warnings from vendored
  Hyphenation and tiny_http. The audit reviewed the same 625 locked packages.
- The primary test run used the command below. It passed 1,335 library tests
  and 310 integration tests, with nine library tests ignored. One archive test
  hit `No space left on device` during extraction. A serial attempt before
  reclaiming space also encountered the storage limit. The native-boundary
  target passed 94 of 94 after reclaiming superseded build artifacts and adding
  test-owned archive workspace cleanup. The distinct library and integration
  total was 1,646 passed.
- `target/debug/kujo test --runtime vm` and
  `target/debug/kujo test --runtime dual` passed 150 of 150 each. Dual used
  the VM for all runnable fixtures. Each strategy reported 11 inventory skips.
- The unsafe budget contract passed at 275 executable occurrences. Generated
  artifact freshness and the strict Rust advisory audit passed.

Primary test command and the two final targeted reruns used
`CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1`.

```sh
cargo test --locked --no-fail-fast --lib \
  --test agent_project_contracts --test cli_contracts --test cli_json_contracts \
  --test diagnostics_golden --test docs_examples \
  --test generated_artifact_freshness_contract --test native_api_security_boundaries \
  --test package_module_workflow_integration --test process_lifetime_contracts \
  --test readme_contracts --test schema_vector_hardening_contracts \
  --test unsafe_inventory_contract --test vm_interpreter_parity_surfaces
cargo test --locked --test native_api_security_boundaries -- --test-threads=1
cargo test --locked --test unsafe_inventory_contract --test generated_artifact_freshness_contract
```

Paired benchmark output and the command logs are in the local
`/tmp/kujo-hardening.aEalIr/` evidence directory. This report records the
portable verification commands and benchmark inputs.
