# Post-1.8.1 token, payload and output efficiency

## Scope and baseline

Repository: `kujolang/kujo`; branch: `main`; starting SHA:
`13f005f7152dc7d228c841142c15d8516fea2af2`. Ending implementation SHA: `9bd525a87216b55ee11ad6a36b071ce1127802a6`.
The subsequent documentation commit records this report and regenerated inventory. This is roadmap item 4, not
a release. Kujo remains 1.8.1.

Reviewed core token estimation/context selection (`token.rs`), AI request hashing,
payload construction, cassette record/replay, tool-call normalization, response
envelopes, redaction and error previews (`http.rs`), JSON conversion (`builtins.rs`),
CLI JSON renderers, the release gate, and MCP delegation (`mcp_make.rs`). Existing
research under `docs/research/token-efficiency/` remains historical evidence, not
a claim about current sibling repositories. No sibling repositories were changed.

Before editing, `cargo test --lib` passed **952 tests**, with zero failures and
seven existing ignores. The starting commit also has the preceding full-gate
receipt in the item-3 audit. This round's fresh baseline is the library suite,
not a claimed fresh full-gate baseline. The first debug-symbol build was stopped
to conserve disk; the successful run used the environment below.

## Findings and decisions

| ID | Priority | Area | Evidence | Action/status |
| --- | --- | --- | --- | --- |
| E4-1 | P1 | Request buffers | Both public hashes and cassette keys serialized a complete canonical JSON string solely to feed SHA-256 | Stream the same validated JSON tree through the same serializer into a buffered digest writer; implemented |
| E4-2 | P2 | Response payload copies | `extract_tool_calls` cloned the contents of immutable ID/name/argument strings while the raw response retained the originals | Share their `Arc<String>` storage; implemented with large-string and fallback contracts |
| E4-3 | P2 | Command output | Release gate sent every successful test/compiler line to the agent/CI console | Explicit opt-in concise receipts with complete private per-command logs; implemented |
| E4-4 | P2 | Regression budgets | Existing token correctness tests did not pin a representative fitted payload's exact byte size | Add synthetic retained-context and receipt-output budgets with an explicit update procedure; implemented |
| E4-5 | Reviewed | Context selection | Current fitter validates every message, estimates once, drops oldest eligible entries, preserves system/latest user and reports an unsatisfiable budget | Preserve policy; new byte/estimate fixture supplements existing exhaustive reference tests |
| E4-6 | Reviewed | Schemas and policy | Core transports `options.body`; MCP command delegates generation to the MCP package | Do not prune tool schemas, handoffs or prompts inside the runtime; policy belongs to SDK/agent owners |
| E4-7 | Reviewed | Output compatibility | Raw JSON, stream chunks, normalized tool calls and structured errors are public results | Preserve fields and previews; no default compact response or silent truncation |
| E4-8 | Reviewed | Cassette buffers | Record path converts through serialized JSON; replay reads the existing complete cassette format | Preserve in this round: direct-tree substitution requires numeric round-trip compatibility analysis; arbitrary new file ceilings would reject existing inputs |
| E4-9 | P1 | Credential previews | A synthetic credential crossing character 240 survived as `credential...`; focused regression failed before the fix; independent review found overlapping credentials can invalidate later replacements | Redact original-text credential spans before truncation; overlapping spans merge with bounded per-credential state; fixed |
| E4-10 | P1 | Error metadata | `Retry-After: 9223372036854775807` panicked on `seconds * 1000` in a focused debug test | Saturating millisecond conversion; normal/negative/boundary values covered; fixed |
| E4-11 | P2 | Verification maintenance | Full gate found the October 3 VM mismatch inventory eight days old at UTC rollover | Canonically regenerated; only date and runner path changed |

## Implemented contracts

`canonical_json_hash` keeps `kujo_value_to_json` validation, canonical key order,
number formatting, secret redaction, and existing caller-specific error prefixes.
An 8,192-byte `BufWriter` feeds SHA-256. The complete JSON tree still exists; this
is not zero-copy serialization or a total-memory ceiling. Public hash version 1
and committed cassette keys remain unchanged. Tests compare against the previous
`to_json`-then-hash path for Unicode, escaping, buffer boundaries, floats, secrets,
duplicate fixed-dictionary keys, nesting boundaries and rejected values.

Tool-call normalization shares existing string storage; missing fields and
non-string argument serialization keep their prior behavior. Tests retain the
source and output simultaneously, assert shared storage for 1 MiB arguments, and
check normalized field values and object/missing-field fallbacks. No strings are
truncated and no provider payload is rewritten.

`KUJO_RELEASE_GATE_LOG_DIR` is an additive **development-script** environment
variable. A unique private run directory prevents collisions and accidental log
overwrites. Command stdout/stderr are captured together, exact exit status is
returned, and `set -e` still stops the gate. Log-path failure stops before commands
run. Default gate output and runtime CLI contracts are unchanged. Full diagnostics
remain local and retrievable; no upload or external service is added.

## Measurements and ratchets

The paired manual hash test retains the previous implementation as an oracle,
alternates order, discards two warmups and records eleven samples per variant.
It checks digest equality on normalized requests with ASCII message content. This
isolates serialization and hashing; it is not end-to-end request latency. It is ignored in ordinary tests and
has **no timing CI threshold**. See the committed measurement receipt for actual
samples in [`evidence/payload-efficiency.json`](evidence/payload-efficiency.json).

Legacy/streamed medians were 16,530/16,567 ns (1 KiB content),
2,159,416/2,185,502 ns (256 KiB), and 19,459,630/19,232,364 ns (2 MiB).
Samples overlap; no latency speedup is claimed. The release measurement binary
preceded the final error-preview sink refinement; the measured hash path and
measurement function were unchanged. Final-source behavior is covered by the
full debug gate.

| Dimension | Before | After | Interpretation |
| --- | --- | --- | --- |
| Hash serialization buffer, normalized request with 2 MiB ASCII content | 2,097,337 serialized bytes retained | 8,192-byte buffer | Measured old string length vs configured bounded buffer; excludes JSON tree, allocator overhead and digest |
| Normalized 1 MiB argument string | Additional 1,048,576-byte content copy | Shared original string | Source-backed copy size and pointer-sharing regression; not a process RSS claim |
| Synthetic chat/context | 345 input JSON bytes / 80 estimated tokens | Existing fitter returns 185 bytes / 30 estimated tokens at budget 40 | Behavior baseline, **not a new savings claim** |
| Gate output fixture | 100,050 console bytes | 208 console bytes | All 100,004 command bytes retained in files; measured with the receipt's relative log path |
| Same console text, `ai_count_tokens` | 25,013 estimated tokens | 52 estimated tokens | Actual deterministic estimator calls; synthetic output, not provider billing |
| Provider billing / wire payload | Unchanged | Unchanged | No provider-exact token or billed-cost claims |
| Dependencies | Existing Rust/Node/Bash dependencies | Unchanged | No new runtime or build dependency |

Tests run through ordinary `cargo test`, so existing CI executes the byte/context,
hash, sharing and shell-output ratchets. Growth is deliberate: edit fixture and
expectations, review retained evidence, recompute byte/heuristic counts, document
the reason and rerun compatibility tests. The canonical procedure is
[`tests/fixtures/context_budget/README.md`](../../tests/fixtures/context_budget/README.md).

## Security and compatibility

Reviewed secret redaction, hash-excluded credential headers, strict replay before
network access, serializer nesting/finite-number validation, tool argument data,
and filesystem log publication. Existing capability gates, endpoint policy,
network timeouts, response limits and cassette publication stay in place. Log
directories are private; logs may contain diagnostics and require appropriate
retention. No validation, error or evidence is removed to reduce bytes.

The error review reproduced two existing bugs before patching. HTTP legacy strings
and structured HTTP/decode previews could retain a credential fragment because
truncation preceded exact-value redaction. The shared excerpt helper now redacts
first. Independent boundary investigation and candidate review also identified
sequential replacement of overlapping credentials as a bypass; matching original
text and merging overlaps closes that route without retaining a per-match list.
Prefix-table matchers avoid repeatedly scanning long self-overlapping credentials;
the heap retains only the next match per credential. Extra matching state scales
with credential lengths/count, not the number of occurrences in the response.
The preview sink retains at most 240 characters plus an ellipsis, avoiding a
complete redacted-response allocation just to crop it. Tests compare this bounded
sink with full redaction followed by the previous truncation oracle, including
large inputs and boundaries inside a replacement marker. Full cassette/error
redaction continues to retain its complete required output.
Tests cover API keys, mixed-case sensitive header names, long credentials,
contained/crossing/self-overlapping credentials, multibyte prefixes and unchanged
secret-free boundaries. Existing metadata,
provider-code redaction and legacy decode messages remain covered. A maximal
integer retry header also reproduced arithmetic overflow; saturating multiplication
preserves ordinary and negative values while making extremes deterministic.

Public API signatures, CLI exit codes, JSON result shapes, schemas, cassette/file formats,
runtime configuration and runtime environment variables do not change. The
development gate gains only the opt-in variable above. External consumers receive
identical hashes and live successful response contents. Credential masking in
errors and stored cassette bodies is intentionally stronger for overlapping values.
Error previews remove previously exposed fragments, and overflowing retry metadata
now returns the maximum integer rather than panicking/wrapping. No new provider
control plane is added.

## Verification receipt

Use `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`
for this host's disk-constrained debug checks. Logs are retained under
`.audit-evidence/payload-efficiency/`; final concise evidence is committed alongside
this report. Exact checks:

```bash
cargo test --lib
cargo test --lib streamed_request_hash
cargo test --test token_payload_budget --test release_gate_output_contract
cargo test --lib ai_error_previews_redact_credentials_before_character_truncation
cargo test --lib retry_after_seconds_handles_the_entire_integer_range
cargo test --lib ai_error_redaction_merges_overlapping_original_credential_spans
cargo test --lib interpreter::native_functions::http::tests
CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --release --lib request_hash_payload_measurement -- --ignored --nocapture
KUJO_RELEASE_GATE_LOG_DIR=.audit-evidence/payload-efficiency/gate bash scripts/release_gate.sh --full
KUJO_MISMATCH_FIXTURE_TIMEOUT_SECONDS=60 KUJO_MISMATCH_CRYPTO_FIXTURE_TIMEOUT_SECONDS=300 bash scripts/generate_vm_runtime_mismatch_inventory.sh --strict --runner target/debug/kujo
KUJO_RELEASE_GATE_LOG_DIR=.audit-evidence/payload-efficiency/final-gate bash scripts/release_gate.sh --full
KUJO_RELEASE_GATE_LOG_DIR=.audit-evidence/payload-efficiency/accepted-gate bash scripts/release_gate.sh --full
cargo fmt --check
git diff --check
```

The three focused bug regressions each failed before their fix. The first full
gate stopped at the existing date-based inventory freshness rule; regeneration
changed only the date/runner metadata. Neither these failures nor exploratory
scalar-only hash timings are presented as accepted final verification.

Final-source full gate passed (exit 0): hygiene, formatting, all-target/all-feature
Clippy, the full Cargo suite (3059 passed, zero failures, 20 ignored across targets),
native security, package integration, VM/interpreter parity, both runtime sweeps,
and dependency audit. Library baseline grew from 952 to 958 passing tests; the
additional ignore is the deliberately manual measurement. The dedicated HTTP
suite passed 42 tests with one manual ignore.

Optional socket-bound tests and generic benchmark smoke were not enabled; the
focused release hash benchmark ran separately. Optional cargo-deny is unavailable.
The existing build-only bincode maintenance-advisory exception remains unchanged.
No new failure was accepted. This is local macOS verification, not a claim of a
new hosted cross-platform campaign.

## Remaining and cross-repository work

No cross-repository change is required. Schema pruning, provider-exact billing,
adaptive context selection and handoff policy stay with their existing ecosystem
owners; this pass does not create new downstream tasks. Whole-process peak memory
and production provider latency were not measured. Further cassette buffering
changes need compatibility evidence; they are not prerequisites for this bounded
item. No new P0/P1 issue is claimed. Item 5 begins with `yield from` design and
compatibility evidence, not automatic grammar changes.
