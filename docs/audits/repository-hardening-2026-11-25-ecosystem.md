# Kujo ecosystem hardening pass

## Repository and baseline

- Repository `kujolang/kujo`, branch `main`, starting commit `aeab2b220d81c003116d4bb24cb594bb09b3710a`.
- Verified implementation ends at `97e998afa31ce3e9affc1bf239a8ec38d9e65515`.
- Kujo provides the Rust lexer, parser, VM, interpreter and native runtime, with CLI, editor, package, AI and documentation surfaces. The published ecosystem installer downloads runtime and source artifacts, and the main website publishes a matching copy.
- Reviewed recent passes covering [runtime lifetimes and transport](repository-hardening-2026-11-21-followup.md), [CLI and editor contracts](repository-hardening-2026-11-21-cli-editor.md), [function calls](repository-hardening-2026-11-22-function-speed.md), and [builtin performance](repository-hardening-2026-11-25-builtin-speed.md). This pass addresses remaining lexer and installer paths and records its own results.

## Findings

| ID | Priority | Area | Evidence and implementation | Status |
| --- | --- | --- | --- | --- |
| KUJO-01 | P0 | Installer credentials | Manifest downloads accepted arbitrary HTTPS destinations through the authenticated GitHub helper. Authorization is now scoped to supported GitHub hosts with HTTPS-only redirects, with a stubbed transport test. | Fixed in `c03152db` |
| KUJO-02 | P0 | Release installation | An explicitly selected ref calls the release install function in a Bash conditional, which suppresses implicit `errexit` inside the function and its callees. A failed extraction or copy could proceed with incomplete data. Temporary download paths also survived exits. | Fixed in `e783d6fb`. Explicit failures terminate, download staging is cleaned on exit, and binary staging renames a complete file into place. |
| KUJO-03 | P1 | String lexer | Each ordinary scalar triggered a scan of its accumulated UTF-8 prefix, even after the size diagnostic was emitted. Plain literals also copied their decoded contents after construction. | Fixed in `97e998af`. Incremental decoding preserves the existing segment boundary, diagnostic ordering and recovery. Plain tokens take ownership of the decoded text. |

## Compatibility

Installer flags, manifest refs, versions and receipt shape are preserved. Authenticated external manifest URLs must be hosted at a supported GitHub origin or supplied as local files. HTTPS manifest URLs remain usable without a token. The installer falls back to source when an explicitly chosen release download is unavailable. An invalid downloaded or staged release now fails without replacing the current binary. The installed executable path is published by rename after permissions are set.

Lexer token types, decoded escapes, interpolation, source locations and diagnostics remain unchanged. Length checks occur at the existing ordinary-character boundaries, including the existing behavior when a segment ends in escaped characters. Parser limits, native capabilities, schemas, configuration, dependencies and environment-variable names are unchanged. The website mirror has the reviewed installer.

## Performance

The [harness](evidence/kujo-string-lexer-2026-11-25.py) compiles the actual baseline and candidate lexer modules with Rust 1.96.0 and optimization level 3 on Intel macOS. It tokenizes each roughly 768 KiB input four times per process. The harness checks decoded byte totals, warms each binary once, and alternates order over five measured pairs. These are lexer harness CPU times, including process startup and file reading. Whole CLI latency and peak memory were outside the measurement.

| Literal input | Baseline median CPU ms | Candidate median CPU ms |
| --- | ---: | ---: |
| ASCII 512 | 123.76 | 50.33 |
| ASCII 4096 | 389.08 | 41.79 |
| ASCII 8192 | 730.04 | 43.95 |
| Unicode 8192 | 674.63 | 21.58 |

[Raw paired samples](evidence/kujo-string-lexer-2026-11-25.jsonl).

## Coverage

| Phases | Evidence |
| --- | --- |
| 0–1 | Read the agent guide, prior audit evidence, Cargo features, release scripts, lexer and affected runtime entrypoints. Collected baseline source, native test results and optimized lexer timings. |
| 2–4 | Traced repeated UTF-8 scanning, plain token allocation, staged source and executable paths and failure cleanup. Retained the lexer source-size guard and limits. |
| 5–6 | Existing runtime audit covers bounded command previews, strict AI replay, normalized request hashing and MCP or handoff delegation. The changed lexer and installer paths do not add model requests, schema registration, replay or handoff traffic. |
| 7–9 | Checked lexer recovery and single diagnostics, installer URL authority, checksum enforcement, extraction and publication. Binary replacement uses a same-directory staging path. Source trees still use the installer’s backup and ref-marker flow. |
| 10–11 | Read public language and CLI compatibility rules, Cargo and installer platform/tool dependencies, and release-state configuration. No dependency changes were needed. |
| 12–13 | Added scalar-count and recovery checks, mocked installer release success and failure, and wired installer coverage into the existing release-state workflow. |
| 14–16 | Updated installer usage and changelog. Preserved the established agent entrypoints, fixtures and prior comparison evidence. No removal was justified by this pass. |
| 17–18 | Implemented findings in independent commits and ran the verification below. |

## Verification

- `bash -n install.sh`, `bash -n tests/install_release_manifest.sh`, and `bash tests/install_release_manifest.sh` passed. The shell suite exercised a successful fixture installation, an extraction failure after a partial file was created, a failed executable copy, cleanup, credential scope, and immutable pins.
- `bash .github/scripts/check-release-state.sh`, `bash .github/scripts/check-contract-version-sync.sh`, `cargo fmt --check`, and `git diff --check` passed.
- A standalone Rust test harness compiled the checked-out `src/lexer.rs`, `src/errors.rs`, and `src/runtime_limits.rs` with the existing Cargo `colored` and `serde_json` libraries. All 15 lexer tests passed, including multibyte limits, decoded escapes, interpolation segment reset, single diagnostics and recovery. The optimized paired benchmark also passed.
- The local baseline Cargo run passed 184 tests across `cli_contracts`, `cli_json_contracts`, `diagnostics_golden`, `docs_examples`, and `vm_interpreter_parity_surfaces`. The baseline library test build and candidate CLI link failed for lack of disk space. This run consequently has no complete local Cargo candidate matrix. [Release-state CI](https://github.com/kujolang/kujo/actions/runs/45745910922) and [runtime CI](https://github.com/kujolang/kujo/actions/runs/45745910947) were queued for the implementation revision when this report was written.
- The website mirror at `00cf691` passed shell syntax and profile listing, and matched canonical `install.sh` byte for byte. [Website publication](https://github.com/kujolang/kujolang.ai/actions/runs/45745683739) was still in progress when this report was written.

## Follow-up

Keep the published installer in sync with canonical `install.sh`. Source-tree replacement still uses portable backup and move operations. A full install transaction spanning sources, shims and the executable would require a separate compatibility design.
