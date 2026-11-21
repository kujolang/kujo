# Kujo CLI and editor hardening, 2026-11-21

## Repository and baseline

This pass started on clean `kujolang/kujo` main at
`9315c657f7f2fad6dc3bb5b9739145c7a1a96465`, after the [previous follow-up](repository-hardening-2026-11-21-followup.md).
Kujo 1.10.0 ships a Rust-hosted language, VM and interpreter, a command-line
toolchain, and a stdio language server. The changes use the existing Serde JSON,
TOML, cap-std, fs2 and tempfile dependencies. The verified source SHA is
`bdc062851f27b996d313718e00f43c4c406e057d`.

On macOS 26.6.2 x86_64 with Rust 1.96.0, `cargo fmt --check` and
`cargo audit --deny warnings` passed on the base SHA. The baseline
`cargo test --locked --lib` passed 1,336 tests with nine ignored.
Two concurrent `package-add` invocations on a 6,000-dependency fixture both
exited successfully but left only one added dependency in each of three
trials. The package source read, validation, edit, and truncate/write occupied
separate uncoordinated operations.

## Findings and implemented changes

| ID | Priority | Evidence and impact | Correction and regression coverage |
| --- | --- | --- | --- |
| E01 | P0 | `package-add` lost successful updates from overlapping processes and published by truncating the manifest. | Serialize the complete transaction with an OS-backed lock in the resolved manifest directory. Stage and sync the replacement before publishing it. Six simultaneous CLI writers, a symlinked manifest, Unix mode bits, and existing comment-preservation tests cover the supported paths. |
| E02 | P1 | The CLI parser bounded the regular file metadata size, then read the input to EOF. The bound did not hold while reading a stream or a file that changed size. | Check metadata on the opened handle and read at most 1,048,577 bytes. Keep the original parser exit code and diagnostic code, and report an observed lower bound if the size was unknown. Verify stream, exact-boundary, and oversized-file behavior through the CLI. |
| E03 | P1 | The LSP body had an 8 MiB ceiling. The header reader could read ahead without a header budget, accepted repeated length fields, and could mistake incomplete headers for clean EOF. | Bound header blocks to 8 KiB and reject incomplete or duplicate lengths. Unit tests assert bytes consumed, exact boundary acceptance, clean EOF, and case-insensitive single-field framing. |
| E04 | P1 | The synchronous LSP dispatcher retained IDs indefinitely when cancellation arrived after the matching request had already completed. | Retain up to 1,024 IDs and 8 MiB of string ID data, with oldest-first eviction. Test count and byte pressure, duplicate IDs, consumption, and one-time cancellation followed by ID reuse. |
| E05 | P1 | Semantic tokens rescanned the line prefix for every token. Symbol ranges, reference ranges, edits, and diagnostics also rescanned prefixes for each coordinate. | Advance semantic tokens through ordered byte offsets and use a sparse index of supplementary characters for other outgoing ranges. Tests compare coordinates against direct prefix counts, including CRLF, BMP and supplementary characters. |
| E06 | P2 | The LSP retained independent copies of open file text in the editor analysis and the module source override, then copied open text again for individual requests. | Share one immutable source allocation between the analysis and module override and borrow it for requests. The existing edit, close, and transitive import tests cover snapshot behavior. |

## Compatibility and trust boundaries

Package manifest syntax, CLI success text, package and LSP result shapes, CLI
exit codes, configuration, and environment settings retain their existing
contracts. The persistent `.kujo-package-add.lock` lives in the resolved
manifest directory, so the directory must be writable. The lock must stay in
place while writers may be using it. Atomic publication retains mode bits and
updates the resolved symlink target. It replaces the target file identity.
Other hard links therefore continue pointing to the original file. An editor
that does not take the sidecar lock can still race with Kujo. Staging syncs
the replacement file before rename. Directory sync and crash recovery were
outside this pass.

The CLI limit applies to the existing `kujo` command entrypoints that already
checked it. The trusted lean runner, runtime module loader, and editor buffer
policies have their own entrypoints. Regular oversized inputs still report
the known size. Unknown-size inputs report the observed lower bound.

Stdio payload limits and valid UTF-16 positions retain their existing
contracts. Malformed framing now fails the transport. Pending cancellations
expire by the documented resource limits. No third-party integration needs
a migration. Package locks are advisory. Kujo's existing trust model and
runtime capability defaults still apply.

## Coverage

| Phases | Evidence and scope |
| --- | --- |
| 0–1 | Checked the clean ref, stable runtime, feature graph, platform workflows, previous audits, CLI and LSP entrypoints, and current test contracts. Ran a locked library and RustSec baseline. |
| 2–4 | Traced source buffer ownership, ordered lexer offsets, UTF-16 range conversion, bounded file reads, and retained LSP state. Measured the two editor request paths below. |
| 5–6 | Checked normalized AI request hashing, strict replay before transport, bounded tool-loop steps, and the MCP delegate's profile/artifacts-only modes. Editor diagnostics, logs, and JSON transport retain their result formats. Schema generation and handoff format belong to the delegated MCP package. Provider tokens and tool schema size were unmeasured. |
| 7–9 | Followed manifest reads through publication, scope of the directory lock, symlink resolution, UTF-8/EOF failure semantics, and deterministic cancellation eviction. Sampled existing capability defaults, HTTP destination checks and limits, upgrade download limits, and native process ownership. No network listener or tool invocation contract changed. |
| 10–11 | Checked package and LSP contracts, dependency ownership, locked dependencies, and advisory policy. Reused dependencies. |
| 12–13 | Added deterministic boundary and behavior assertions to existing suites, and ran the established checks. No timing threshold was added. |
| 14–16 | Aligned language and editor docs with implementation, including the lifecycle behavior for an early full-text `didChange`. Kept documented public and lean-runner behaviors and removed redundant open-source copies. No deprecated feature or dependency had enough evidence for removal. |
| 17–18 | Implemented the fixes above. See measurements and verification below. |

## Measurements

On an Intel i7-9750H host, five baseline/candidate pairs ran in alternating
order for each workload. Both builds were unoptimized with debug information
disabled. Each process read a temporary UTF-8 file via its file URI, served one
stdio request, then handled shutdown and exit. Timings include process launch,
lexing, JSON encoding, and pipe transfer.

| Workload | Baseline median and range | Candidate median and range |
| --- | ---: | ---: |
| Semantic tokens for `print("😀",1);` repeated 1,500 times on one line | 2,786.80 ms (2,636.80–2,992.43) | 52.67 ms (47.61–60.37) |
| Document symbols for `let item := "😀";` repeated 2,000 times on one line | 3,364.05 ms (3,327.23–3,574.71) | 174.06 ms (171.39–207.75) |

Each semantic response contained 4,500 tokens. Its framed stdout SHA-256 was
`02788b30e4dde2059b30d1c50c008a57c9793d32bea1379fed493f1969f022df`
in every run. Each document-symbol response contained 2,000 symbols and matched
`ad05b6e45807819d3b00e781cae1627053a664509261e35300afa84ae6769a70`.
These workloads measure long-line request latency. Peak RSS and release-build
latency were unmeasured.

## Verification

The Cargo test and compile commands used
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1`.
The two test matrices passed 1,451 distinct tests, with nine ignored and no
failures. Parser diagnostics ran in both matrices and are counted once.

```sh
cargo test --locked --no-fail-fast --lib --test parser_diagnostics_contract \
  --test lsp_conformance_harness --test lsp_external_clients_smoke \
  --test lsp_latency_guardrails --test lsp_reliability_track \
  --test cli_json_contracts --test diagnostics_golden --test docs_examples \
  --test readme_contracts
cargo test --locked --test package_module_workflow_integration \
  --test parser_diagnostics_contract --test cli_contracts
```

| Command | Result |
| --- | --- |
| `cargo fmt --check`, `git diff --check` | Passed |
| `cargo check --locked` | Passed |
| `cargo check --locked --no-default-features` | Passed with feature-gated warnings |
| `cargo clippy --locked --lib --bin kujo -- -D warnings` | Passed with existing vendored dependency warnings |
| `cargo audit --deny warnings` | Passed against 1,555 advisories and 625 locked packages |
| `bash scripts/repo_hygiene_audit.sh` | Passed |
| `bash scripts/generate_unsafe_inventory.sh --strict --output-md /tmp/kujo-hardening-9315/UNSAFE_INVENTORY.md --output-csv /tmp/kujo-hardening-9315/UNSAFE_INVENTORY.csv` | Passed. Both outputs matched the tracked files byte for byte |
| `target/debug/kujo test --runtime vm` | Passed 150 runnable fixtures, with 11 inventory skips |
| `target/debug/kujo test --runtime dual` | Passed 150 runnable fixtures, with 11 inventory skips and zero interpreter fallbacks |
| `KUJO_AI_REPLAY=tests/fixtures/ai_cassettes KUJO_AI_REPLAY_MODE=strict target/debug/kujo run examples/ai_enterprise_replay_showcase.kujo` | Passed with strict replay |
| `python3 -u /tmp/kujo-hardening-9315/paired_lsp_bench.py /tmp/kujo-hardening-9315/kujo-baseline target/debug/kujo` | All response digests matched |

Raw local test and benchmark receipts are under `/tmp/kujo-hardening-9315/`.

## Remaining scope

The local matrix covers the affected CLI and editor paths, with runtime smoke
tests and strict AI replay. The repository-wide integration matrix and native
platform release qualifications require a separate run. There is no known
introduced regression or cross-repository implementation blocker.
