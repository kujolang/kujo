# Changelog

This file records user-visible changes to Kujo. It follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- MySQL transaction control uses the text protocol, as required by MySQL 8.4,
  while parameterized application queries continue using prepared statements.

- PostgreSQL TLS session initialization and cleanup now stay outside an active
  Tokio executor, preventing nested-runtime panics from async interpreter code.
- Registered the existing `db_last_insert_id` builtin with the type checker,
  eliminating an incorrect undefined-function warning.

- `db_close` now releases SQLite, PostgreSQL and MySQL resources immediately,
  invalidates retained aliases, and rolls back uncommitted work. Repeated close
  remains successful; operations on closed or returned pool leases fail explicitly.
  Pool returns preserve the native session but invalidate the old lease so stale
  aliases cannot affect a later borrower. See `docs/DATABASE_LIFECYCLE.md`.
- MySQL connections now use Kujo's persistent async executor instead of a reactor
  destroyed after each operation, fixing queries against a shut-down runtime.

## [1.5.0] - 2026-09-22
- Added `digest_file_beneath` for bounded streaming SHA-256 of rooted regular
  files up to 64 MiB without retaining the full artifact in memory.

- Added `read_stdin(max_bytes)` for bounded, exact UTF-8 process input through EOF.

- Added bounded, descriptor-relative `list_dir_beneath` pages with explicit scan ceilings, suffix-based cursors, and symlink/race coverage.

### Changed

- Added `copy_file_beneath`, a bounded streaming, no-follow copy with atomic no-replace publication and a SHA-256/byte receipt. Both filesystem read and write capabilities are required.
- Added `sha256_file_beneath` for bounded streaming SHA-256 through no-follow
  rooted file handles, returning the digest and actual byte count up to 4 GiB.
- Added capability-gated `list_dir_page(path, after, limit, suffix)` for sorted
  directory pages retaining at most limit+1 filenames. Each page still scans
  the directory; entry/UTF-8 errors are explicit and no cross-call snapshot is
  promised.
- Streaming AES encryption/decryption now publish with atomic no-replace
  semantics, so concurrent writers cannot overwrite a successful output.
  Additive receipt fields expose publication and temporary-file cleanup facts.

- Added `read_binary_prefix_beneath` for bounded regular-file prefix reads
  through a rooted handle, retaining in-root relative symlinks and rejecting
  concurrent symlink escapes.

- Added opt-in `kujo run --isolated-imports` and inherited
  `KUJO_ISOLATED_IMPORTS=1` for installed tools: imports use entry-file and
  explicit module roots without discovering the caller's modules or lockfile,
  while filesystem operations retain the caller's working directory. Isolated
  runs preserve exact script arguments; default import behavior is unchanged.
- Added bounded, in-process `pdf_render_html` and atomic
  `pdf_render_html_to_file` APIs for branded business documents without an
  external renderer.
- Added `secure_random_token(byte_length)`, an OS-CSPRNG-backed, base64url,
  redacted token primitive isolated from deterministic random seeding.
- Added `db_pool_postgres_tls`, a bounded verified-TLS PostgreSQL pool with
  warm-up, deadlines, health checks, connection expiry, session reset, clean
  shutdown, and operational counters.

- Added lowercase `header_values` arrays to routed HTTP server requests in both
  runtimes, preserving duplicate headers for explicit application validation.

- Expanded scheduled fuzz smoke coverage from the lexer, parser, and XML parser
  to every maintained bounded decoder target, including gzip and ZIP.
- Added a weekly RustSec audit that rejects vulnerable locked dependencies.
- Updated package documentation to distinguish Kujo 1.0's historical local-only
  boundary from the public Kennel registry available today.

### Fixed

- Interpreter top-level function calls no longer read or overwrite unrelated
  caller-local bindings. Direct, indirect, callback, and pipe calls preserve
  lexical boundaries while retaining global assignment and captured closures.



- VM routed HTTP dispatch now invokes closures created by imported modules,
  allowing modular route registrars to capture application context with the
  same behavior as interpreter mode.
- Kept the source-bound TCP security regression server open until the client
  closes, removing a startup-speed race on slower debug builds.
- Restored the runtime mismatch inventory to 149/149 VM matches by adding
  charset snapshots, making integer membership checks explicitly Boolean, and
  excluding the credentialed PostgreSQL TLS probe from offline inventory runs.

## [1.4.0] - 2026-09-09

### Added

- Added `kujo run --scheduler-no-timeout` for long-lived services supervised by
  another process.
- Added `kujo run --isolated-imports` and inherited import isolation for installed
  tools. Isolated runs use the entry file and explicit module paths instead of the
  caller's project, while preserving exact script arguments.
- Added the bounded web and artifact primitives needed by native Kujo tools:
  HTML tokenization, URL handling, text decoding, streaming XML, JSON/JSONL file
  operations, digests, private staging, and atomic no-replace publication.
- Added POSIX file locks, ownership checks, atomic symlink updates, exact process
  replacement, and confined atomic file publication for native package installers
  and command launchers.
- Added `byte_length(value)` for the UTF-8 byte length of strings and raw bytes.
- Preserved repeated HTTP response headers in `http_request.header_values`.

### Fixed

- Streaming AEAD now creates private output files.
- `delete_file` can unlink symlinks, including dangling ones, without touching
  their targets. It still rejects real directories.
- String `contains` keeps its integer `1`/`0` result, while collection membership
  is inferred as Boolean. User and imported signatures still take precedence.
- Windows no-replace directory publication now preserves UTF-16 paths, rejects
  embedded NULs, and atomically refuses existing destinations.
- VM loop scopes now initialize fresh `let` and `const` values and unwind cleanly
  on control flow, returns, and exceptions.
- Async map workers now keep independent state and immutable capture metadata.
- Cross-runtime callbacks now preserve globals, captures, capabilities, errors,
  and bounded recursion without cloning unrelated bytecode state on each call.
- Regex helpers now reuse compiled expressions through a bounded cache.
- Explicit DNS pinning and deny-private HTTP clients now ignore ambient proxies,
  preventing a proxy from resolving the destination again.
- Binary file fixtures, non-JIT parity tests, and bounded web-data regressions now
  cover the shipped behavior instead of skipping or stopping early.

## [1.3.1] - 2026-09-06

### Fixed

- Prevent transient Linux `Text file busy` errors from aborting verified native
  runtime upgrades. Staged version checks retry only this condition, sharing
  one deadline across retries and process execution. Other execution errors,
  checksum verification, version matching and destination checks remain intact.

### Changed

- Exercise inherited writable executable descriptors and repeated parallel
  native upgrades in Linux CI. Verify clean, lifecycle-script-disabled npm
  installations on every supported native target.

## [1.3.0] - 2026-09-05

### Added

- Add native runtime-only `kujo upgrade [VERSION]`, exact stable release selection,
  read-only `--check`, documented `--json`, explicit downgrade opt-in, official
  SHA-256 verification, managed-install guidance, and retained recovery binaries.
  Upgrade archive support remains available without default language features.

- Add database-capability-gated `db_connect_postgres_tls` for PostgreSQL over
  mandatory TLS 1.2+ with peer and hostname verification, an explicit bounded
  public CA bundle, TCP-host enforcement, and secret-free deterministic
  connection failures.

- Add capability-free `decode_charset(bytes, charset, max_output_bytes)` with
  strict no-replacement decoding, explicit 64 MiB resource ceilings, exact
  MIME ISO-8859-1 semantics, registered legacy charset support, and
  VM/interpreter parity.

- Add filesystem-read-gated `decode_text_file_range_info` to compose strict
  identity/Base64/quoted-printable streaming with strict registered-charset
  conversion, independent decoded/UTF-8 hashes, fixed memory, explicit output
  and prefix ceilings, and VM/interpreter parity.

- Add `db_connect_readonly("sqlite", path)` for capability-gated, fail-closed
  immutable inspection of existing checkpointed SQLite databases without
  create, write, WAL or shared-memory side effects.

- Preserve doctor-profile arguments, bounded structured reports up to 1 MiB,
  and optional readiness/profile data through generic Doctor composition.

- Add routed, authorization-preflighted HTTP uploads with fixed-memory body
  streaming into auto-cleaned private files, caller-lowerable 64 MiB ceilings,
  SHA-256 receipts, read deadlines, and interpreter/VM parity. Upload routes
  require explicit network-server, filesystem-write, and filesystem-delete
  capabilities and never expose partial files to application handlers.

- Add `publish_file_noreplace` for atomic same-filesystem adoption without
  overwriting an existing destination. It requires filesystem-write and
  filesystem-delete capabilities and returns explicit publication, unlink,
  directory-durability, inode-identity, byte-count, and verification facts.

- Add bounded incremental response streaming to generic `http_request`, with
  transparent routed-server pass-through, response headers, chunk lifecycle
  callbacks, callback cancellation, downstream-disconnect reporting, and
  interpreter/VM server support without full-response buffering.

- Add bounded TCP/TLS file-range streaming, transformed file-range private
  spools, tracked SQLite immediate transactions, and expanded IP-literal facts.
- Add strict per-request destination policies, DNS pinning, redirect controls,
  and bounded timeouts for streaming HTTP file uploads and downloads.

### Fixed

- Keep synchronous PostgreSQL operations safe when called from async workers
  and preserve PostgreSQL parameter types.
- Run interpreter execution outside the CLI async runtime to avoid nested
  runtime failures for synchronous host APIs.
- Refresh generated TODO and unsafe inventories for the shipped source tree.


- Capture the lexically nearest same-named local in VM closures. This prevents
  a closure created in a later block from binding an inactive earlier block's
  slot, while preserving interpreter parity.

- Accept VM fixed-dictionary option literals in TLS client and server APIs,
  preserving the same allowlist, size limits and minimum-version validation as
  interpreter dictionaries.

- Make imported async Kujo functions callable through both VM call paths with
  interpreter-equivalent arity, captured environment and promise behavior.

- Bound the shared async runtime to one scheduler worker instead of inheriting
  host CPU count. Nonblocking tasks still interleave and explicitly bounded
  blocking/Rayon lanes retain parallelism without host-dependent stack growth.

## [1.2.3] - 2026-09-02

### Added

- Add capability-free `parse_xml_bounded` with namespace-aware deterministic
  trees, caller-lowerable absolute resource ceilings, VM/interpreter parity,
  a dedicated cargo-fuzz target, and fail-closed DTD, non-predefined entity,
  namespace, and malformed-document handling.

- Add a filesystem-read-gated, fixed-memory file-range decoder for strict
  identity, Base64 and quoted-printable transfer encodings. It returns a
  bounded inspection prefix, decoded length, SHA-256, UTF-8/ASCII/NUL facts
  and input-line evidence without returning the decoded body.

- Add stable `response_code` and `name_exists` fields to bounded DNS lookup
  envelopes so Kujo programs can distinguish `NXDOMAIN` from an existing
  name with no records of the requested type.

- Add capability-gated, bounded streaming canonical CRLF text-range hashing
  and normalized RSA public-key inspection so protocol implementations can
  verify large signed artifacts without unbounded buffering or exposing key
  parsing through protocol-specific runtime code.

- Add descriptor-relative text and binary reads that reject symlinks, reparse
  points, special files, traversal, replacement races, and size-limit drift
  beneath an explicitly authorized filesystem root.

- Add bounded string/byte stdin to structured process execution, staged in a
  private auto-deleting file and reopened read-only before process spawn.

### Fixed

- Point the cargo-fuzz harness at the renamed `kujolang` package while keeping
  the library import alias `kujo`, restoring all configured fuzz targets.

- Give both CLI and shared interpreter async runtimes a fixed 8 MiB worker
  stack so bounded protocol and policy functions can run under `promise_all`
  without overflowing Tokio's smaller default worker stack.
- Run the complete CLI lifecycle on a dedicated large-stack thread so Windows
  does not overflow its smaller default main-thread stack before VM or
  interpreter execution.

### Security

- Close post-validation filesystem replacement races for applications that
  read files selected beneath a trusted root.
- Let applications pass already-authorized bytes to structured argv processes
  without shell interpolation, unbounded pipe writers, writable inherited
  stdin handles, or hidden capability expansion.

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
- Add stable `encode_uri_component(text)` RFC 3986 UTF-8 percent encoding and strict `decode_uri_component(text)` decoding for provider and web integrations.
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
