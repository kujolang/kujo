# Kujo HTTP ingress hardening

## Repository and baseline

This follow-up to the [runtime security pass](repository-hardening-2026-11-26-security.md) began on `kujolang/kujo` `main` at `9e330b0ec1c5b22244baf87361a337786ca809f1`. The implementation ends at `6384264c985cd2c35fd1caf8e0804f4b32be0c3f`. Kujo's Rust CLI serves static files and dispatches routed HTTP requests in both the VM and interpreter. All three paths share the vendored tiny_http parser. Kujo uses its OpenSSL adapter, while the vendored crate also has a Rustls adapter.

On Intel macOS with Rust 1.96.0, baseline formatting passed. All 33 static HTTP integration tests passed using the compact binaries built by the prior pass. A fresh `cargo audit --json` reported zero vulnerabilities and zero warnings for 625 locked crates. The default-profile workspace test was stopped under disk pressure. A separate cached library run was incomplete and reported failures in AI HTTP tests, so it did not provide a clean current full-suite baseline.

## Findings and fixes

| ID | Priority | Area | Evidence | Action | Status |
| --- | --- | --- | --- | --- | --- |
| HTTP-01 | P0 | Request-head resources | `ClientConnection::read` accumulated request lines and fields before the static handler applied its limits. Routed servers shared the parser. The socket timeout covered individual reads. | Bound request-line bytes, combined field bytes, field count, and total elapsed read time in the parser. | Fixed in `020fc32e` |
| HTTP-02 | P0 | Inbound framing | `new_request` used the first length and coding fields, treated an invalid length as absent, and chose the chunked decoder for any transfer coding. | Validate field syntax on ingress and select body framing only when every declaration agrees. | Fixed in `6384264c` |
| HTTP-03 | P0 | Chunk metadata | The locked `chunked_transfer` 1.5.0 decoder accumulates chunk size metadata before returning decoded data. Application payload limits apply after that allocation. | Bound chunk metadata in the body decoder and preserve chunked upload coverage. | Open, SignalBox `sig_a159ef68-4f38-43a5-b18e-8a5e5df4b689` |
| HTTP-04 | P0 | HTTPS admission | OpenSSL negotiation runs synchronously on the listener's accept thread under socket timeouts. | Isolate negotiation in bounded admission and apply a total handshake deadline. | Open, SignalBox `sig_0cf0794c-f183-4eed-b924-301028779080` |

The transport applies the existing static flags. Routed servers use defaults of 8,192 line bytes, 16,384 header bytes, and 100 fields. The parser counts wire bytes and ends the connection on a 414, 413, or 408. It starts a fresh deadline when it obtains the reader for each request and restores the socket timeout before body consumption. Early failures receive an empty transport response before handler access logging. Existing valid fixed-length and chunked requests remain supported, including identical length declarations. Invalid field syntax and ambiguous framing receive HTTP 400 and close the connection. [RFC 9112, message body length](https://www.rfc-editor.org/rfc/rfc9112.html#section-6.3)

The new parser checks have no file-format, JSON schema, dependency, CLI-flag, or environment-variable changes. Proxies and clients that previously generated ambiguous framing must send an unambiguous request. No cross-repository change is required. The earlier keep-alive interaction with long-running handlers remains recorded separately in SignalBox. The request-head deadline begins when the next reader is ready, so this pass does not change that connection lifecycle.

## Checklist coverage

| Phases | Scope and evidence |
| --- | --- |
| 0–1 | Compared the recent audit with shared parser code, entry points, transport dependencies, tests, and release gates. Recorded baseline formatting, existing static behavior, and the advisory check. |
| 2–4 | Removed reconstructed request-head checks in the static handler. Inspected parser allocation, socket waits, chunked metadata and TLS admission. No performance benchmarks were run. |
| 5–6 | Reviewed strict AI egress and replay controls, process output limits and redaction, and the agent scaffold's explicit tools, handoffs, and retry settings. Model-visible output and schema budgets were outside this HTTP ingress pass. |
| 7–9 | Traced parser error responses, ordered reusable request readers, timeout restoration, and pre-dispatch body framing. Also reviewed the existing handle-relative static file and strict DNS boundaries. |
| 10–11 | Checked static flag semantics, routed settings, and pinned transport dependencies. The locked supply-chain scan was clean. |
| 12–13 | Added vendored transport unit coverage for exact-limit body handoff, expired read budget, and accepted and rejected framing. The static CLI and routed suites provide application-level checks. |
| 14–16 | Updated security posture, CLI help, vendoring notes and the changelog. Broader cleanup, token efficiency, and dead-code removal were outside the security scope. |
| 17–18 | Applied independent parser-resource and framing fixes. See verification below for platform and execution limits. |

## Verification

- `cargo fmt --check`, `git diff 9e330b0e..HEAD --check`, `bash .github/scripts/check-release-state.sh`, and `bash .github/scripts/check-contract-version-sync.sh` passed.
- `cargo audit --deny warnings --no-fetch` passed with the refreshed local advisory database.
- `rustc --edition 2018 --test --crate-name tiny_http --cfg 'feature="ssl-openssl"'` against the Cargo-built locked dependencies passed all 15 vendored unit tests. The standalone Cargo manifest uses a separate dependency set and stopped when the filesystem could not write its package cache.
- `rustc --edition 2021 --test` with the production `serve_http.rs` and `path_security.rs` modules passed all 32 module tests. The unchanged `tests/serve_command_integration.rs` passed all 33 cases against a temporary executable compiled from the production serve command and modules with locked dependencies.
- Vendored `input-tests`, `network`, and `non-chunked-buffering` passed all 17 integration cases via `rustc --edition 2018 --test` against the Cargo-built parser. These checks include HTTP/1.0, pipelining, and ordinary fixed-length input.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --offline --locked -j 1 --test serve_command_integration --test http_route_callback_closure --test http_route_upload --test http_route_concurrency -- --test-threads=2` stopped with `ENOSPC`. Moving temporary output to memory-backed scratch also failed when Cargo tried to write to the host target directory. The production CLI build, routed suites and the full workspace matrix remain unverified on this host. The Rustls adapter and non-macOS platforms were outside the executed verification.
