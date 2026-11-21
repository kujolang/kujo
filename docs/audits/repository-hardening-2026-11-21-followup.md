# Repository hardening follow-up on 2026-11-21

## Repository

`kujolang/kujo`, branch `main`, was clean at
`7863d1e2305179d6cb6fb9a7cee5fc58662e9048`. This is a separate pass from the
[earlier November audit](repository-hardening-2026-11-21.md). Kujo 1.10.0 is a
Rust-hosted language with a bytecode VM, interpreter, native I/O, local package
tooling, and deterministic AI record and replay. The changed paths integrate
with the vendored tiny_http server, Rayon, Serde JSON, and the already locked
TOML parser and editor. The last source commit is `27767b5a`.

## Baseline

On macOS x86_64 with Rust 1.96.0, `cargo fmt --check` and the strict RustSec
audit passed. The targeted locked Cargo matrix passed 1,410 tests with nine
ignored. The initial unrestricted Cargo test began recompiling dependencies
with the default debug profile and was stopped before a test verdict. The
targeted matrix reused debug-info-free builds given the space available.

A real `kujo package-add` run against a temporary manifest removed both a
comment and its `[tool.workflow]` table. A loopback TCP test linked against the
vendored server built from the base SHA timed out waiting for the failed
response's keep-alive connection to close. The same test linked against the
patched server passed. The SSG benchmark below compiled the relevant source
definitions from each version before timing them.

## Findings

| ID | Priority | Area | Finding and evidence | Action | Status |
| --- | --- | --- | --- | --- | --- |
| F01 | P0 | HTTP response integrity | The chunk encoder wrote its terminator from `Drop` after a read error. `Request::respond` released its sequential writer on error while the socket stayed open. A downstream reader could wait on a failed response and a later response could flush the terminator. | Write the terminator after a clean EOF and shut down the shared socket before yielding the sequential writer on failure. | Fixed |
| F05 | P0 | HTTP request resources | The destructor for an unread fixed-length body allocated a scratch buffer sized to the remaining body on every read. | Discard unread bytes in fixed 8 KiB chunks and preserve the exact read boundary. | Fixed |
| F02 | P0 | Manifest integrity | `add_dependency` parsed into a reduced schema and rewrote the entire TOML document. A CLI reproduction lost unrelated metadata and comments. | Validate the manifest under the existing stable schema and edit only the chosen dependency in its original document. | Fixed |
| F03 | P1 | Runtime resources | The fused SSG path kept two page-name vectors per observed file count and a separate Rayon pool per thread count for the lifetime of the process. | Retain one eligible page-name batch and one worker pool, with independent ownership for in-flight work. | Fixed |
| F04 | P1 | AI replay and artifacts | `Path::exists` collapsed metadata errors into cache misses before opening a cassette. The recorder randomized header order and created an extra normalized request string and full pretty-printed cassette string. | Treat an absent cassette as a miss, retain I/O errors, order response headers, and stream pretty JSON into the existing staged file. | Fixed |

## Implementation and compatibility

- **HTTP.** The vendored tiny_http path preserves per-chunk flushing. On a body
  or output error, `respond_impl` closes the connection while it still holds the
  current response's sequential writer. The first candidate retained an extra
  writer reference into unread request cleanup. The existing upload integration
  test caught the resulting close timeout. Moving the reference into
  `respond_impl` releases it before request-reader teardown. Discarding an
  unread fixed-length request body now needs only an 8 KiB scratch buffer,
  independent of the amount remaining. Clean responses keep their normal
  framing. The loopback check observes the exact body bytes for success and for
  a reader that fails after its first chunk. Kujo's configured upstream timeout,
  byte ceiling, capability checks, and HTTP request schemas are unchanged.
- **Package editing.** `package_workflow.rs` uses the existing locked version of
  `toml_edit` to preserve metadata and dependency comments. CLI coverage checks
  a normal table, dotted declarations, and an inline table, including a no-op
  update and insertion of a dependency whose literal name contains a dot.
  Manifest validation, lockfile schema, CLI status text, and exit codes are
  unchanged. The new direct dependency was already present transitively, with
  the same package versions and no new locked packages.
- **SSG.** The fused builtin caches one page-name pair for up to 65,536 pages
  and reuses one Rayon pool. Larger batches build and use their own names.
  Eviction preserves existing references, and the pool cache serializes
  creation so competing requests for the same size can reuse one pool.
  Tests check reuse, oversized work, eviction, active pool validity, and
  existing checksums. No builtin argument, output field, or concurrency limit
  changed.
- **AI.** Cassette schema version 1 and the request hash are unchanged. The
  writer retains the same redaction and atomic replacement path. A failed
  cassette lookup only falls through on `NotFound`. Strict replay remains
  network-free. Tests cover header order, credential redaction, replaying the
  result, absent and unreadable paths, and existing concurrent recordings.

There are no new runtime settings, environment variables, or external consumer
migrations. No other Kujo repository needs a change.

## Performance and efficiency

The isolated SSG probe compiled the cache functions and HTML literals from the
base and candidate with `rustc --edition=2021 -O` and linked the same Rayon
1.12.0 build. It generated and hashed all page names for 10,000 through 32,000
pages in steps of 2,000, then for 70,000 pages. It next performed 500,000
lookups for the same 32,000-page batch and created pools from one through
eight workers. Each run used a fresh process. The three baseline and candidate
runs alternated and all returned the same page-name hash (`36edc8d72f51ab29`).

| Measurement | Baseline | Candidate |
| --- | ---: | ---: |
| Page-name strings retained after the varied workload | 644,000 | 64,000 |
| Workers in cache-retained pools after the sizing workload | 36 | 8 |
| Median of three varied-workload timings | 286.410 ms | 467.921 ms |
| Median of three same-size lookup timings | 62.986 ms | 37.615 ms |

The varied workload includes name construction, hashing, and releasing evicted
batches. It took longer with eviction. The lookup workload measures reuse
without file I/O. These are resource counts and isolated timings, with no
end-to-end SSG throughput or RSS claim. Cassette recording eliminates two
full serialized string buffers while keeping the normalized JSON tree and
redacted response. Its wall time and peak RSS were not measured. Provider
tokens and tool-schema sizes were not measured.

## Review coverage

| Phases | Implementation inspected and scope |
| --- | --- |
| 0–1 | Checked repository and release state, previous audit, Cargo features and lockfile, VM and interpreter entrypoints, CLI contracts, test suites and fixtures, and release and advisory workflows. Ran the baseline format, audit, and targeted test matrix. |
| 2–4 | Traced typed manifest serialization, SSG page metadata and worker lifetimes, byte streaming and chunk framing, and cassette buffers. Measured retained resources, steady reuse and varied batches. |
| 5–6 | Checked AI normalized request hashing, strict and fallthrough replay, bounded tool-loop steps, and cassette serialization. `mcp_make.rs` exposes profile-only and artifacts-only switches and delegates tool registration, schemas, context packaging and handoffs to the ecosystem package. Reviewed CLI JSON and diagnostic surfaces, with no output-shape or prompt changes. |
| 7–9 | Followed EOF, failed upstream reads, pipelined writer ownership, socket shutdown, cassette open and publication, SSG eviction and active ownership, process capture from the prior pass, and shared promise completion. Checked native capability defaults, network destination policy, response bounds, and existing credential and rooted filesystem controls. The socket fix targets integrity of incomplete responses, and replay preserves the distinction between local failure and a permitted network fallthrough. |
| 10–11 | Checked public signatures and schemas, CLI statuses, dependency metadata, locked versions, and the advisory gate. Retained the vendored patch surface and existing dependencies for supported build configurations. |
| 12–13 | Added behavior checks to existing test suites and reused generated artifact freshness and unsafe budgets. No elapsed-time threshold was added. |
| 14–16 | Aligned language, native runtime, and HTTP docs. Used guide links and structured contracts for progressive disclosure. Reviewed benchmark-specific helper seams and the prior audit's cleanup; no further removals had sufficient compatibility evidence. |
| 17–18 | Implemented the five findings. See the receipts below for verification and limits. |

## Verification

The baseline test command used
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1`.

```sh
cargo test --locked --no-fail-fast --lib \
  --test package_module_workflow_integration --test promise_completion_contracts \
  --test cli_contracts --test cli_json_contracts --test readme_contracts \
  --test generated_artifact_freshness_contract --test unsafe_inventory_contract
```

The same environment was used for the candidate Cargo test commands.

```sh
cargo test --locked --no-fail-fast --lib \
  --test package_module_workflow_integration --test promise_completion_contracts \
  --test cli_contracts --test cli_json_contracts --test readme_contracts \
  --test generated_artifact_freshness_contract --test unsafe_inventory_contract \
  --test docs_examples --test http_route_callback_closure \
  --test serve_command_integration --test vm_interpreter_parity_surfaces \
  --test http_pinning_proxy_contract --test http_route_concurrency \
  --test http_route_upload --test native_api_security_boundaries
```

The first candidate passed the 1,582-test primary matrix, with nine ignored.
The supplemental network matrix then found the upload close regression.
The original upload test passed against the base runtime and failed on two
runs against the first candidate. The final combined rerun passed 1,684
tests, including both upload contracts, with nine ignored. A separate serial
run also passed both upload contracts.

```sh
cargo fmt --check
cargo check --locked
cargo clippy --locked --bin kujo -- -D warnings
cargo check --locked --no-default-features
cargo audit --deny warnings
bash scripts/generate_unsafe_inventory.sh --strict
bash scripts/repo_hygiene_audit.sh
git diff --check
rustc --edition=2018 --test vendor/tiny_http-0.12.0/src/util/equal_reader.rs \
  -o /tmp/kujo-hardening-7863/equal-reader
/tmp/kujo-hardening-7863/equal-reader
target/debug/kujo test --runtime vm
target/debug/kujo test --runtime dual
KUJO_AI_REPLAY=tests/fixtures/ai_cassettes KUJO_AI_REPLAY_MODE=strict \
  target/debug/kujo run examples/ai_enterprise_replay_showcase.kujo
```

Formatting, default and minimal compilation, strict binary Clippy, RustSec,
unsafe inventory generation, repository hygiene, and the whitespace check
passed. Both Kujo test strategies passed 150 runnable fixtures, with 11
inventory skips each. All three standalone request-cleanup tests passed. The AI showcase passed in strict replay. The standalone loopback
wire probe passed on plain TCP and OpenSSL. A separate TCP probe observed
the response EOF before unread request cleanup finished. Default builds emit
the existing vendored warnings, and the minimal build emits feature-gated
warnings.

The full integration matrix, platform release jobs, and live provider
qualification were outside this local verification. No dependency advisory,
contract change, or cross-repository blocker was found. Local logs and standalone
benchmark source are under `/tmp/kujo-hardening-7863/`.
