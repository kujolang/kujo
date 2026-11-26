# Kujo runtime security hardening

## Repository and baseline

Kujo is a Rust-built runtime and CLI for Kujo scripts, with native host-effect APIs, a static preview server and a universal documentation generator. This security-focused pass began on `kujolang/kujo` `main` at `7e0c25694ff03696ba3269dc9b69f0cf6f5ced79` and the implementation ends at `ea79793b5c815aa5fea5041ec6a5322e55ac3022`. The affected dependencies are `url` for URL parsing, reqwest and Tokio for connection setup, `cap-std` and `cap-fs-ext` for directory-relative file access, and the vendored `tiny_http` server. The review used Rust 1.96.0 on Intel macOS.

Baseline `cargo fmt --check` and the five DocGen gap unit tests passed. The initial default-debug `cargo check --locked` stopped at a local disk-space error. A broader baseline library test run was stopped to reclaim local build space for the candidate. The fresh RustSec audit flagged one locked dependency advisory. The earlier offline audit passed against an older advisory database, so the fresh audit is the baseline for dependency comparison.

## Findings and fixes

| Priority | Area | Evidence and correction | Status |
| --- | --- | --- | --- |
| P0 | URL origin handling | The lockfile selected `url` 2.5.8, and `url_components` calls the affected upstream `Url::origin` on caller-provided input. Raised the direct minimum and lockfile to 2.5.9. Added a native contract test covering tuple and opaque origins. [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2026-0592). | `a801346e` |
| P0 | DocGen outbound requests | `external_link_check` performed its private-address lookup separately from reqwest's connection and used an ambient-proxy client. The default client now checks the address set passed to the connector, uses direct transport and checks literals separately. The same bounded redirect loop and diagnostic code cover every allowlisted hop. | `161c1d1e` |
| P0 | Static file reads | `build_response` and the custom 404 path validated canonical paths before opening them. The final open still used an absolute pathname, and ETags re-read metadata through that pathname. The server now opens canonical path components from the startup directory handle without following symlinks. File size and ETag use metadata from the opened file. The same file-opening path applies to compressed variants and custom 404 pages. | `292ce4a5` |
| P0 | Static socket timeouts | The static server's receive queue used the read timeout. The HTTP transport was started without either configured socket timeout. Both settings now apply to accepted sockets before TLS and request parsing. The CLI integration test verifies a 408 response when a connection reaches the configured read timeout. | `ea79793b` |

## Compatibility and coverage

DocGen still uses one HTTP client per validation run and checks the explicit host allowlist at every redirect. The default mode uses direct transport, expands private-address checks to match the runtime's handling of unspecified and IPv4-mapped addresses, and preserves the dedicated diagnostics and the `--allow-private-network-links` opt-in. `url_components` preserves its response fields and returns the string `"null"` for opaque origins. The patched dependency makes `blob:` origins opaque when the inner scheme falls outside the upstream standard's supported set. The only additional lockfile update is `form_urlencoded` 1.2.3, required by `url` 2.5.9. Both libraries declare a Rust version below Kujo's 1.89 minimum.

Static content retains canonical path policy, ranges, ETags, precompressed selection and custom 404 handling. The startup directory is the authority for file opens. The server still expects the configured root and its contents to be administered by a trusted operator. The existing timeout flags now apply to HTTP and HTTPS sockets. No CLI flags, exit codes, schemas, file formats, environment variables or external Kujo integrations were changed. No other repository requires an update.

| Checklist phases | Evidence |
| --- | --- |
| 0–1 | Reviewed repository instructions, recent audit reports, feature gates, release workflows and the security posture. Ran baseline formatting, a compact test build and the current dependency audit. |
| 2–4 | Traced duplicate address lookup across DocGen and reqwest, checked network timeouts and the existing client reuse test, and compared pathname metadata with opened-file metadata. Reviewed request, archive and subprocess output bounds. |
| 5–6 | Reviewed strict cassette replay and redaction, structured workflow output limits and MCP/agent entrypoints for exposure and amplification. The patch leaves model requests, handoffs and tool schemas outside its scope. |
| 7–9 | Reviewed resolver failure mapping, redirect policy, filesystem confinement, handle lifetime, and credential/cassette publication. Tests cover default private-address diagnostics and static reads anchored to the original directory handle. |
| 10–11 | Checked public DocGen and native URL contracts, declared Rust minima, the locked dependency diff and the existing release audit. |
| 12–13 | Added regressions to existing test suites and regenerated the unsafe inventory line references. Existing release jobs provide test and dependency gates. |
| 14–16 | Updated DocGen, native security posture and standard library documentation. Removed the duplicated DocGen IP policy. Broader cleanup, performance and token budgets were outside the security scope. |
| 17–18 | Committed four independent fixes and compared the affected tests and dependency audit against the baseline. |

No throughput or memory benchmarks were run. Existing regression coverage checks one client per DocGen validation run. The default path has one DNS resolution per new connection and reuses allowed connections through reqwest's pool.

## Remaining work

The shared HTTP parser still needs request-head resource limits before application dispatch (P0). Static-server line, byte and count limits presently run after HTTP parsing. An absolute parser budget and compatible error handling need design and verification for both static and routed servers. The socket timeout change bounds individual I/O waits.

## Verification

- `cargo fmt --check`, `git diff --check`, `bash .github/scripts/check-release-state.sh`, and `bash .github/scripts/check-contract-version-sync.sh` passed.
- `cargo audit --deny warnings` passed against the refreshed RustSec database after the dependency update. The VS Code extension's `npm audit --omit=dev --package-lock-only --audit-level=low --json` reported zero production dependency vulnerabilities.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 3 --lib --test docgen_universal --test serve_command_integration --test unsafe_inventory_contract --test stdlib_reference_contract -- --test-threads=4` passed. The library reported 1,350 passed and 9 ignored. Integration suites reported 55 DocGen, 33 static server, 3 unsafe inventory and 5 standard-library tests passed.
- `target/debug/kujo test --runtime vm` and `target/debug/kujo test --runtime dual` each passed all 150 runnable fixtures with 11 configured skips.
- Local verification covered Intel macOS. The full workspace integration matrix and other platforms were outside the local run.
