# Repository hardening round two — 2026-10-10

Repository: `kujolang/kujo`, branch `main`.
Starting SHA: `0c6f9f892886e285b2c6a09473db97bed700cfb2`.
Ending implementation SHA: `3a375f7f7d6b4ea3f4a11b2eca6e97491fd1ca13`.
The following documentation-only receipt commit records this verified implementation.
Purpose: the Rust runtime, CLI, interpreter, and VM for the Kujo language.
Relevant integrations: OpenAI-compatible message protocols, strict local replay,
`serde_json` serialization, and Rayon vector scoring. Dependency versions are unchanged.

This pass targets reproducible correctness bugs, repeated native work, and
model-visible diagnostic volume. It does not assume that the reported approximate
bug count is accurate. The previous audit and Windows/GIF follow-up remain in
[`repository-hardening.md`](repository-hardening.md).

## Baseline and evidence

The starting worktree was clean. Cargo initially failed before running tests
because the volume was full. `cargo clean --profile dev` removed 17.6 GiB of
rebuildable development artifacts; source and previous audit evidence were kept.
The baseline was restarted with incremental compilation disabled. Raw receipts
are retained locally in `.audit-evidence/hardening-2026-10-10/`.

The unchanged baseline passed **2,988 tests, 0 failures, 18 ignored** across
104 reported test/doctest receipts. The ignores are existing policy/opt-in tests;
none were added or changed by this pass. The existing vendored `tiny_http`
compiler warnings also predate this work.

The initial `hardening_round_two` regression target failed **12/12 tests twice**,
in 0.19 and 0.14 seconds. It then passed **12/12** after the initial fixes. Tests
were subsequently organized into collection, schema/vector, and AI-history
contract targets with a shared fixture helper. A further literal-based CLI
smoke exposed R211: VM exit 4 versus two successful interpreter assertions.
The same VM failure was repeated before applying the dictionary-view fix.
The original failure receipt is preserved
[here](evidence/round-two/reproductions-before.txt). R211's ordinary-dictionary-only
parser is also present at the starting SHA; it was not introduced by the initial fixes.

## Findings

Ten correctness bugs and two efficiency issues are resolved below. Findings are
grouped by root cause rather than counted per affected API; all regression and
existing verification gates passed.

| ID | Priority | Area | Finding | Evidence / reproducer | Action | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R201 | P1 | Collections | Integer `sum` uses unchecked addition | `sum_overflow_is_a_runtime_error_instead_of_a_panic_or_wrap` | Checked accumulation | Fixed; regression verified |
| R202 | P1 | Collections | `unique` uses redacted Debug text as identity | `unique_does_not_treat_redacted_debug_output_as_value_identity` | Verify equality on summary collisions | Fixed; regression verified |
| R203 | P1 | Collections | Specialized integer dictionaries are copied instead of inverted | `invert_supports_every_integer_dictionary_representation` | Reuse primitive inversion | Fixed; regression verified |
| R204 | P1 | Vector math | Squared norms overflow/underflow for finite inputs | `vectors_preserve_direction_at_large_and_small_finite_scales` | Scaled norms; reuse query norm | Fixed; regression verified |
| R205 | P1 | JSON Schema | `const`/`enum` inherit approximate language equality and lossy integer conversion | `schema_const_and_enum_use_exact_json_numeric_equality` | Exact bounded structural comparison | Fixed; regression verified |
| R206 | P1 | AI protocol | Input normalization drops tool-call correlation metadata | `ai_request_hash_preserves_tool_call_linkage` | Preserve validated metadata | Fixed; regression verified |
| R207 | P1 | AI protocol | Tool loop drops assistant calls and sends replies without IDs | `ai_tool_loop_replays_a_protocol_valid_two_step_conversation` | Retain calls and correlate replies | Fixed; regression verified |
| R208 | P2 | JSON Schema | Local JSON pointers cannot traverse arrays | `schema_local_json_pointers_can_address_array_elements` | Traverse canonical array indices | Fixed; regression verified |
| R209 | P2 | Diagnostics | Numeric conversion errors echo arbitrarily large rejected strings | `numeric_conversion_errors_bound_model_visible_input_echoes` | Shared 128-character preview | Fixed; regression verified |
| R210 | P1 | Collections | Mixed numeric sorting rounds large integers before comparison | `sort_does_not_round_large_integers_before_comparing_floats` | Share exact mixed-number comparator | Fixed; regression verified |
| R211 | P1 | Runtime parity | AI parsers reject VM-specialized message/content dictionaries | `ai_dictionary_literals_and_multimodal_blocks_work_in_both_runtimes`; repeat CLI smoke | Borrow both dictionary representations | Fixed; regression verified |
| R212 | P2 | Token helpers | Eager normalization and option copies do work for discarded/count-only messages | Paired probe; exhaustive context-fit equivalence tests | Borrow input; normalize only retained output | Fixed; measured and verified |

The tool-call contract is supported by the official
[tool-message schema](https://github.com/openai/openai-python/blob/main/src/openai/types/chat/chat_completion_tool_message_param.py)
and [assistant-message schema](https://github.com/openai/openai-python/blob/main/src/openai/types/chat/chat_completion_assistant_message_param.py),
read on 2026-10-10. Tool replies require `tool_call_id`; assistant history carries
the corresponding `tool_calls`. These are data fields, never executable commands.

## Changes implemented

### Collection values and exact ordering

`sum` now uses checked integer addition and returns `Value::Error` without a
panic, wrapping, or mutation of input. Previously the VM panic surfaced as exit
6; the corrected failure is the normal runtime exit 4. Float promotion and the
established treatment of non-numeric elements are preserved.

`unique` previously treated `Debug` text as identity. Debug deliberately prints
only array lengths, dictionary sizes, byte lengths, and secret redactions. It
therefore collapsed distinct values with the same summary. The hash lookup is
retained, but collisions are checked against actual values. Primitive lookup
remains the ordinary fast path; no new caches or retained global state exist.
Separate CLI reproductions returned `1,1,1` for distinct arrays, dictionaries,
and bytes in both runtimes before the fix.

Integer dictionary variants now feed the existing inversion implementation,
including its primitive-value conversion and unsupported-value rules. Mixed
numeric sorting uses the exact integer/float comparator formerly confined to
schema validation. It handles the i64 boundaries before casting, and preserves
NaN's unordered behavior. Language-wide equality is deliberately unchanged.

Files: `collections.rs`, `value.rs`,
`tests/collection_hardening_contracts.rs`.

### JSON Schema

`const` and `enum` now compare nested JSON numbers mathematically instead of
using Kujo's compatibility epsilon comparison or rounding an i64 through f64.
`1` and `1.0` remain equal; `9007199254740993` and `9007199254740992.0` do not.
An explicit comparison worklist shares the existing depth/node budgets, so
constant and enum comparisons cannot bypass those bounds. Local JSON pointers
can traverse arrays using canonical decimal indices; object-key escaping and
remote/cyclic-reference rejection are preserved.

Files: `schema.rs`, `value.rs`,
`tests/schema_vector_hardening_contracts.rs`.

### Vector math

Squared finite components could overflow to infinity or underflow to zero before
taking a norm. Scaled norms avoid that intermediate loss. Normalization divides
by scale and scaled length, preserving direction even for subnormal components.
Cosine compares scaled vectors; top-k computes the query norm once for all rows.
Zero vectors, stable index tie breaks, input limits, and errors for genuinely
unrepresentable norm magnitudes remain covered. The previous test labeling
`f64::MAX` cosine as a non-finite result was corrected: its cosine with itself
is 1. Actual infinity rejection and overflowing norm tests remain.

Files: `vector.rs`, `tests/schema_vector_hardening_contracts.rs`.

### AI history and runtime parity

Input normalization preserves names, call records, and correlation IDs. The
same call validator checks input histories and provider responses, rejecting
missing/duplicate IDs and malformed function records instead of treating an
unfinished call as an answer. Tool-loop replies retain each ID even when two
calls have the same function name. Caller-provided tool result strings are
shared rather than copied.

The replay regression independently constructs the canonical expected request
and its SHA-256 cassette key. The first request succeeded before the fix, but
the incorrect second request missed replay; after the fix both requests match.
It uses a closed loopback endpoint and strict replay, with no live provider.

A borrowed dictionary view now validates both ordinary and VM-specialized
dictionaries, including nested image blocks and function calls. Literal-based
CLI coverage protects parity; validation does not clone entire dictionaries.

Files: `http.rs`, `tests/ai_tool_history_contracts.rs`, `docs/AI_RUNTIME.md`.

### Token helpers and diagnostic volume

Token messages and options are borrowed during validation/estimation. Only
retained fitted messages are normalized into output dictionaries; count-only
calls and discarded messages do not construct those dictionaries. Model prefix
selection no longer copies and lowercases the entire model name. Existing
exhaustive role/model/budget equivalence tests and the 100,000-message boundary
remain the behavior oracle. Estimates, pruning rules, input validation, and
output shapes are unchanged.

Four numeric conversion surfaces share a Unicode-safe diagnostic preview: short
messages remain byte-for-byte compatible; longer rejected input shows its first
128 characters and original byte length. No input data is changed. The tests
cover ASCII and multibyte input and both runtime exit paths.

Files: `token.rs`, `type_ops.rs`, `tests/schema_vector_hardening_contracts.rs`,
`scripts/diagnostic_volume_bench.rs`.

## Performance and efficiency

Measured diagnostic message size, with the same 8,192-byte invalid ASCII input:

| Surface | Before bytes | After bytes | Before estimated tokens | After estimated tokens |
| --- | ---: | ---: | ---: | ---: |
| `parse_int` | 8,218 | 176 | 2,055 | 44 |
| `parse_float` | 8,216 | 174 | 2,054 | 44 |
| `to_int` | 8,216 | 174 | 2,054 | 44 |
| `to_float` | 8,218 | 176 | 2,055 | 44 |

Token values are Kujo's deterministic `ai_count_tokens` estimates, not provider
billing counts. Byte counts exclude the CLI's unchanged diagnostic envelope.
The probe calls the actual native APIs; the Unicode test also verifies bounds.

Context fitting: one system message plus N user messages, 64-token budget,
the existing `scripts/token_context_bench.rs` workload. Three alternating paired
trials each contain two warmups and seven timed samples per size; medians below
combine the 21 samples for each version and size.

| N | Before median | After median |
| ---: | ---: | ---: |
| 100 | 0.195 ms | 0.034 ms |
| 1,000 | 5.258 ms | 0.379 ms |
| 5,000 | 29.522 ms | 3.590 ms |

The comparison uses the same optimized included token module, compiler, and
debug support library for both versions. This isolates module work; it is not a
whole-runtime release benchmark. Host scheduling was noisy; all samples are
preserved, without discarding outliers, in
[before](evidence/round-two/token-context-before.txt) and
[after](evidence/round-two/token-context-after.txt). Diagnostic receipts are
[before](evidence/round-two/diagnostics-before.txt) and
[after](evidence/round-two/diagnostics-after.txt).

No RSS, build-speed, binary-size, or dependency-reduction claim is made.
Dependencies and lockfile are unchanged. Vector query-norm reuse is
code-supported work elimination, not a measured wall-clock claim.

## Compatibility and boundaries

- Public signatures, CLI flags, config keys, environment variables, schema result
  fields, and cassette serialization/hash version numbers are unchanged.
- Tool histories now preserve required metadata, so their transmitted bodies and
  corresponding hashes change. Re-record cassettes for those corrected bodies.
  Plain-text cassette/hash fixtures remain compatibility gates.
- Tool-loop `messages` now includes assistant calls and reply IDs. Existing
  fields and caller-provided results by function name remain; no tool execution,
  retries, provider policy, or capability expansion was introduced.
- Large numeric errors are intentionally concise. Schema constant/enum work now
  shares the documented depth/node bounds. Incorrect numeric/collection results
  and overflow panics are corrected as described above.
- Secrets remain redacted in debug output. Deduplication compares values without
  exposing them. Provider content is validated as data, never treated as
  instructions or executable code. New tests use local data and strict replay.
- No sibling repositories were modified and no cross-repository change is
  required. Prior process isolation/converter limits remain documented in the
  previous audit; this round does not broaden the sandbox claim.

## Remaining work

- P0/P1/P2: no confirmed finding from this round remains unresolved.
- Needs more evidence: production memory consumption and provider-billed token
  impact were not measured; do not extrapolate the isolated module probe.
- Not worth changing without a consumer contract: removal of established AI
  response fields solely to reduce payload size. Useful context is retained.
- Cross-repository follow-ups: none required; conditional cassette migration for
  corrected named/tool histories is described above.

## Verification receipt

All commands were run from the repository root unless noted. Raw logs are in
`.audit-evidence/hardening-2026-10-10/`; no test was disabled or marked expected-fail.

| Command | Result / receipt |
| --- | --- |
| `CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast` | Environmental failure before tests: full disk |
| `cargo clean --profile dev` | Removed only rebuildable dev artifacts; 17.6 GiB |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast` | Unchanged baseline: 2,988 passed, 18 ignored, 0 failed; `baseline.log` |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test hardening_round_two` | Twice before changes: 12 failed; after initial fixes: 12 passed; `reproductions-before*.log`, `reproductions-after.log` |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test ai_tool_history_contracts ai_dictionary_literals_and_multimodal_blocks_work_in_both_runtimes` | Passed after R211 fix; `ai-literal-reproduction.log` (verification despite original log name) |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed after replacing two clones in new test fixtures with borrowed slices; `clippy.log` |
| `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked` | Passed; `check.log` |
| `cargo fmt --check` | Passed |
| `rustfmt --check --edition 2021 scripts/diagnostic_volume_bench.rs` | Passed |
| `git diff --check` | Passed |
| `target/debug/kujo test --runtime vm` | 150/150 passed, 11 policy skips; `runtime-vm.log` |
| `target/debug/kujo test --runtime dual` | 150/150 passed, 11 policy skips, 0 fallbacks; `runtime-dual.log` |

Final full-suite command:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast`.
It passed **3,001 tests, 0 failures, 18 unchanged ignores** across **107 receipts**,
including all **13 new regression tests**. An earlier full run is retained as
`final-tests-first.log`; `final-tests.log` is the successful run against the completed R211 code.
The full suite includes documentation/examples, CLI and JSON contracts, diagnostic
goldens, generated-artifact freshness, network/process boundaries, and runtime parity.
The new contract targets are automatically discovered by the existing Cargo test
gate; no flaky wall-clock performance threshold was added.

Additional public CLI checks:

```sh
target/debug/kujo run .audit-evidence/hardening-2026-10-10/ai-history.kujo
target/debug/kujo run --interpreter .audit-evidence/hardening-2026-10-10/ai-history.kujo
target/debug/kujo run .audit-evidence/hardening-2026-10-10/unique-before.kujo
target/debug/kujo run --interpreter .audit-evidence/hardening-2026-10-10/unique-before.kujo
```

The first VM AI check failed and led to R211; the permanent AI literal regression
now exercises both runtimes and nested multimodal content. The unique checks
returned `1,1,1` before and `2,2,2` after in each runtime.

Benchmark builds (same support library for both token versions):

```sh
rustc --edition=2021 -O scripts/token_context_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-2026-10-10/token-before
rustc --edition=2021 -O .audit-evidence/hardening-2026-10-10/token_candidate_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-2026-10-10/token-after
rustc --edition=2021 -O scripts/diagnostic_volume_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-2026-10-10/diagnostics-before
rustc --edition=2021 -O scripts/diagnostic_volume_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-2026-10-10/diagnostics-after
```

The before diagnostic binary was linked before source changes and the after binary
after changes. The token candidate probe changes only the included module path;
`cmp src/interpreter/native_functions/token.rs .audit-evidence/hardening-2026-10-10/token-candidate.rs`
passed. Execute the two token binaries in order before/after, after/before,
before/after for the three paired trials; execute each diagnostic binary once.
The rlib filename is toolchain-specific; a future reproduction must resolve its
own compatible library path. Initial release-library probe attempts were
discarded after a platform LLVM bitcode linker incompatibility and an interrupted
thin-LTO rebuild; no reported timing comes from those attempts.
