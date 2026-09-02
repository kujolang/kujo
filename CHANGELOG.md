# CHANGELOG

All notable changes to Kujo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add stable `response_code` and `name_exists` fields to bounded DNS lookup
  envelopes so Kujo programs can distinguish `NXDOMAIN` from an existing
  name with no records of the requested type.

- Add capability-gated, bounded streaming canonical CRLF text-range hashing
  and normalized RSA public-key inspection so protocol implementations can
  verify large signed artifacts without unbounded buffering or exposing key
  parsing through protocol-specific runtime code.

### Fixed

- Give both CLI and shared interpreter async runtimes a fixed 8 MiB worker
  stack so bounded protocol and policy functions can run under `promise_all`
  without overflowing Tokio's smaller default worker stack.

## [1.2.2] - 2026-09-01

### Added

- Preserve raw bytes for outbound and server-side HTTP request and response
  bodies across interpreter and VM paths.
- Add binary-safe gzip compression for byte payloads.
- Add raw byte concatenation and search primitives.

### Fixed

- Keep Unix-only private-spool metadata checks out of Windows builds, restoring
  the complete native release and npm package matrix.
- Run `npm pack` through the Windows command shell so the native `npm.cmd`
  launcher works during platform-package assembly.

## [1.2.1] - 2026-09-01

### Added

- Add capability-gated bounded private-file spools (`io_private_spool_open`,
  `io_private_spool_write`, `io_private_spool_finish`, and
  `io_private_spool_abort`) with restrictive-at-creation permissions,
  single-use handles, incremental SHA-256 receipts, no-overwrite atomic
  publication, VM/interpreter parity, fail-closed cleanup, and explicit
  post-publication durability/temporary-cleanup facts.
- Add lifecycle-script-free npm runtime packaging with exact-version native
  platform packages, an allow-listed launcher, package contract tests, and
  trusted-publishing provenance.
- Extend `tcp_info` with socket-derived peer/local IP and port fields so raw
  protocol servers can enforce address-scoped admission policy without parsing
  ambiguous combined address strings.

### Fixed

- Honor declared HTTP request-body lengths in routed interpreter and VM
  servers, rejecting declared overflow before reading and responding as soon as
  a complete keep-alive body arrives instead of waiting for the read deadline.
- Reject tag-time binary and npm publication when the Git tag, Cargo version,
  and npm package versions do not match exactly.

### Security

- Close the routed HTTP request boundary so oversized declared bodies are
  rejected before buffering and completed requests cannot be held until the
  read deadline.

## [1.2.0] - 2026-09-01

### Added

- Add capability-free bounded in-memory gzip decompression and safe
  single-regular-entry ZIP reading for hostile compressed protocol inputs.
- Add a checksum-verified reusable GitHub Action for installing exact Kujo
  release versions on Linux, macOS, and Windows CI runners.
- Add deterministic IP-scope classification and structured TCP bind probes for
  network policy checks and server startup diagnostics.
- Add capability-gated, bounded MX, TXT, PTR, and TLSA DNS lookup primitives
  with deterministic result envelopes and DNSSEC proof status.
- Add verified TLS client sockets, server acceptors, consumptive STARTTLS-style
  TCP upgrades, bounded TLS I/O, certificate fingerprints, and TLS 1.2+
  fail-closed protocol policy.
- Add deterministic TCP peer/local-address inspection and bounded per-stream
  read/write timeout controls for protocol servers.
- Add strict `decode_base64_utf8(text)` decoding for text-based binary protocol
  fields without filesystem conversion or lossy UTF-8 behavior.
- Add capability-gated `tcp_connect_bound(host, port, source_ip)` so protocols
  that require deterministic egress-IP selection can bind a validated unicast
  local address before connecting.
- Add per-request `http_request` destination denial, DNS pinning, and redirect
  disabling so untrusted webhook-style callbacks can close DNS-rebinding and
  redirect-based SSRF paths without relying on process-global policy; callers
  can also lower the bounded response-body ceiling per request.

### Changed

- Bound routed HTTP header/body reads with a configurable socket deadline and
  expose direct socket peer address, IP, port, and transport fields with
  matching VM/interpreter behavior.
- Use `kujolang` as the Cargo/crates.io package name while preserving `kujo` as
  the installed CLI command and Rust library crate name.
- Align `parse_json`'s input ceiling with the 8 MiB file-I/O boundary so JSON
  written and read by Kujo remains parseable without an artificial 1 MiB gap.

### Fixed

- Replace the yanked `mysql_async 0.37.0` dependency with `0.37.1` so the
  release dependency audit remains warning-free.

## [1.1.0] - 2026-08-30

### Added

- Add the repository-owned `kujo agent` project lifecycle with deterministic
  profiles, Agent Doctor diagnostics, inspect/run/eval commands, pinned
  ecosystem composition, live AI SDK bridging, Workcell execution, and a
  checked-in self-hosted knowledge-agent example.
- Add first-class `kujo agent auth` credential management backed by macOS
  Keychain, Windows Credential Manager, or Linux Secret Service, with masked
  interactive entry, stdin/CI setup, private project overrides, connector-key
  support, credential readiness diagnostics, and secret redaction.
- Add stable `encode_uri_component(text)` RFC 3986 UTF-8 percent encoding for provider and web integrations.
- Add bounded, streaming `jsonl_query(path, options)` filtering and constant-memory join support for Kujo-native evidence workflows.
- Accept standard JSON Schema Draft 2020-12 identification and annotation keywords in `json_schema_validate`, including `$schema`, `$id`, `$comment`, `format`, `deprecated`, `readOnly`, and `writeOnly`.
- Add a deterministic Kujo-native repository policy gate example with stable
  JSON reports and passing/failing contract fixtures.

### Fixed

- Make Agent project integration fixtures portable to hosted CI by checking out
  every composed ecosystem repository at its exact scaffolded commit.
- Preflight JIT benchmark bytecode before execution so unsupported benchmark
  programs report a bounded result instead of triggering a Cranelift panic, and
  render unavailable runtime speedups as `N/A`.

- Eliminate the remaining Cargo audit maintenance warnings by upgrading
  Cranelift off `region 2`/`mach` and replacing the unmaintained `paste` macro
  used by image-codec dependencies with maintained `pastey` compatibility
  patches while preserving the current image/EXR feature set and parallelism;
  the release gate now denies warnings.
- Remove the `RUSTSEC-2023-0071` RSA timing advisory by moving public RSA
  operations to a current vendored OpenSSL implementation and using AWS-LC for
  the HS256-only JWT backend; the release gate no longer suppresses the advisory.
- Preserve exact integer comparisons in `json_schema_validate` beyond the IEEE-754 safe-integer range.
- Reject incomplete `jsonl_query` join configurations instead of silently returning an empty join.
- Bound `ssg_build_output_paths` before allocating generated path arrays.
- Prevent `spawn_process` stream redaction from exposing secrets split across incremental flush boundaries.
- Make LSP definition, reference, hover, rename, document-symbol, and inlay-hint handling recognize standalone `mut` bindings and lexical scope correctly.
- Preserve CRLF source text during LSP rename edits and accept lexer-supported Unicode identifier continuations.
- Use the LSP-required UTF-16 code-unit coordinate system for incoming positions and outgoing ranges, including semantic tokens.
- Reject LSP messages larger than 8 MiB before allocating their payload buffer.
- Update transitive database and error-handling dependencies to patched releases for `RUSTSEC-2026-0190` and `RUSTSEC-2026-0253`.
- Recognize fallible calls inside multiline `try` blocks in `kujo lint` instead of reporting false missing-error-handling warnings.
- Preserve outer-block indentation after formatting a nested closing brace.
- Ignore braces inside strings and comments when computing formatter indentation.
- Stop definition, reference, hover, and rename lookups from selecting an identifier when the cursor is immediately after it.
- Emit LSP reference ranges that span the complete identifier.
- Emit zero-width LSP edits for missing-delimiter insertion actions.
- Limit range-formatting edits to the client-requested range.
- Limit inlay hints to the client-requested range.
- Include trailing newline positions in full-document formatting edits.
- Consume cancelled request IDs so later requests may safely reuse them.
- Accept the case-insensitive `Content-Length` header names required by the LSP transport protocol.

## [1.0.2] - 2026-08-26

### Added

- Automatically discover locked Kennel-installed dependency roots from the nearest project `kennel.lock`, so normal package consumers no longer need manual `KUJO_MODULE_PATH` wiring.

### Changed

- Preserve `KUJO_MODULE_PATH` as an explicit override/extension point while keeping project-scoped, deterministic package resolution and path-containment protections.

### Security

- Update the locked `h2` dependency to `0.4.16` to address `RUSTSEC-2026-0258`.

## [1.0.1] - 2026-08-11

### Fixed

- Build Linux release binaries on Ubuntu 22.04 so the published artifact remains compatible with Ubuntu 22.04 hosts such as Cloudflare Pages.

## [1.0.0] - 2026-08-08

### Added

- Initial public release of Kujo.
- VM-first language runtime, CLI, LSP, package lockfile workflow, static server, capability controls, and deterministic AI runtime primitives.
- Prebuilt Linux x64, macOS x64/arm64, and Windows x64 binaries with SHA-256 checksums.
- User-local ecosystem installer with core, AI, quality, showcase, and operating profiles.

### Changed

- Finalized machine-readable CLI and runtime diagnostic contract version identifiers at `1.0.0`.

### Fixed

- Repaired all remaining syntax-drifted examples, removed the expected-fail example list, and added exhaustive per-file verification coverage.
