# Native API representation and parity campaign

Scope: roadmap item 2, after the published 1.8.1 maintenance baseline. Runtime
source starts at main `2fa9b02cf303ee21e579708ab043f766aff6cf03`; release source is
`357796a1a868f2c80bc8f7b8edcc7d6aff384f04`. This campaign targets representation
boundaries, not new language features or a claim of exhaustive equivalence for
every possible program. Canonical native names, aliases, arity, and capabilities
remain in `Interpreter::get_builtin_names`, `native_function_arity`, and
`docs/STANDARD_LIBRARY.md`.

## Inventory and executable evidence

| Surface | Accepted representations / limits | Executable evidence |
| --- | --- | --- |
| Dictionary primitives: len, keys, values, items, get, get_default, has_key, invert, clear | Dict, FixedDict, IntDict, DenseIntDict, DenseIntDictInt, DenseIntDictIntFull | native_representation_matrix: all_dictionary_representations_meet_collection_contracts |
| merge / update | All 36 pairs of those six representations; right-hand override, input retention | native_representation_matrix: every_dictionary_pair_merges_and_updates_without_losing_entries; native_representation_parity: merge_update_clear_accept_mixed_dictionary_representations_without_mutation |
| contains | Dictionary string keys return bool; string search retains integer 0/1 | native_representation_parity: integer_dictionary_contains_has_boolean_result; vm_interpreter_parity_surfaces: vm_and_interpreter_preserve_contains_result_types |
| JSON, pretty JSON, TOML, YAML | All dictionary representations, including nested containers; serializers retain existing scalar and unsupported-type rules | native_representation_matrix: all_dictionary_representations_serialize_and_validate; native_representation_parity: specialized_dictionary_serializers_and_schema_preserve_nested_values |
| CSV | Arrays of ordinary/fixed string-key row dictionaries; intentional documented restriction, no implicit byte/compound cell conversion | serialization_schema_hardening |
| Schema instances / const / enum | Dictionary instances are objects regardless of internal specialization; exact numeric comparison, nested constraints, empty specialized schemas and integer-keyed properties/definitions with local refs | native_representation_matrix; native_representation_parity; schema_vector_hardening_contracts; serialization_schema_hardening |
| Arrays and callbacks | map/filter/reduce/find/any/all, take/skip/concat/chunk/flatten/reverse/zip/enumerate | native_representation_parity: collection_callbacks_and_array_operations_share_behavior |
| Bytes, secrets, numbers | Raw byte round trip and slices, redacted nested serialization, exact mixed sorting, bounded schema numbers; JSON bytes intentionally rejected | native_representation_parity: arrays_bytes_secrets_and_numeric_boundaries_preserve_contracts; collection_hardening_contracts |
| AI messages, hashes, context estimates | Ordinary/fixed string-key messages/options; tool linkage and multimodal blocks; hashes and estimates remain deterministic mechanisms | native_representation_parity: ai_construction_hashes_and_token_estimates_accept_literal_and_parsed_objects; ai_tool_history_contracts; ai_replay_hermeticity_contract |
| Sparse and null dictionaries | Only assigned keys exist; null is a present value, not an absent slot; globals, locals, optimized fill loops | native_representation_parity: sparse_integer_writes_do_not_create_phantom_entries, sparse_optimized_fill_retains_only_written_keys, null_dictionary_entries_are_present_and_participate_in_equality |
| Key identity | Only canonical decimal string keys alias integer keys; noncanonical strings remain distinct | native_representation_parity: numeric_looking_string_keys_retain_their_exact_identity |
| Integer helper keys | get/get_default/has_key/remove accept integer or string keys without checker warnings; contains remains string-key-only | native_representation_parity: numeric_helper_keys_are_supported_without_type_warnings_in_both_engines |
| Failure boundaries | Native failures retain exit 4; immutable binding rejection; capability denial precedes filesystem access | native_representation_parity: errors_and_capability_denial_keep_runtime_exit_class; native_api_security_boundaries |
| Host-effect APIs | Existing capability, process, filesystem, database, network, image, archive and PDF suites remain authoritative; no new remote service or provider claims | Full release gate; existing platform/provider workflows |

Dictionary key enumeration retains the established representation-specific
ordering: string maps sort lexically, integer maps numerically. Tests compare
semantic results where order is not a shared contract. TOML null-to-empty-string
behavior is deliberately retained. AI message/options objects require named
string fields; arbitrary integer-key objects are not new message formats.
JIT remains experimental. The matrix runs under default features and without
JIT in the existing parity CI job; it does not promise JIT equivalence.

## Findings and changes

| ID | Priority | Area | Reproduction on 1.8.1 | Root cause / action | Status |
| --- | --- | --- | --- | --- | --- |
| N01 | P1 | Native length | `let size := len; size({"a":1})` errors in VM | Missing FixedDict arm; add native length handling | Verified |
| N02 | P1 | Membership | Integer-assigned dictionary rejected by `contains` | Missing integer storage arms; preserve boolean result and constant-time lookup | Verified |
| N03 | P1 | Clearing | `clear` rejects integer-assigned dictionary | Add all integer storage forms; return the ordinary empty dictionary so it remains usable as options/schema; input retained | Verified |
| N04 | P1 | Merge/update | Mixed integer/literal maps return empty map | Handle mixed representation pairs; share identical merge/update dispatch, retain homogeneous fast paths and legacy invalid-input result | Verified |
| N05 | P1 | Serialization | Integer-only dense dictionaries fail JSON, pretty JSON, TOML and YAML | Add missing specialized conversion cases; reuse existing conversion/redaction rules | Verified |
| N06 | P1 | Schema | Integer-assigned dictionary fails object type validation; integer-keyed properties maps rejected | Normalize specialized instances, borrow schema maps and resolve their local refs; accept empty specialized schemas and retain exact nested const/enum comparison and depth/node budgets | Verified |
| N07 | P1 | Sparse state/resource use | Assigning only key 2 reports length 3; fused fill from 2 reports phantom zero entries | Select sparse storage before global/local/fused writes can resize dense vectors; contiguous fast paths retained | Verified |
| N08 | P1 | Null state | Dictionary with key 0 set to null equals empty map and indexing errors | Treat optional-int None slots as present nulls in equality and index reads; sparse holes are prevented at writes | Verified |
| N09 | P1 | Key identity | `d["00"]` aliases/overwrites key 0; `get(d,"+0",99)` finds 0 | Share allocation-free canonical decimal key parsing across native collection lookup, assertions and VM access; preserve missing-key diagnostics | Verified |
| N10 | P1 | Deduplication | `unique([integer_map,equal_literal])` returns two entries | Dictionary variants now share size-based candidate buckets before semantic equality; nested array case covered | Verified |
| N11 | P1 | Integer helper keys | `get(d,0,99)` returns 7 in VM but null in interpreter; `get_default`/`remove` error | Convert only integer keys for ordinary/fixed dictionaries in get/get_default/has_key/remove; declare string-or-int checker signatures instead of suppressing warnings | Verified |

Changes are confined to `src/builtins.rs`, `src/interpreter/value.rs`, the
collection/string/type/schema native handlers, the type checker, and VM dictionary write/read
boundaries. No compiler grammar, bytecode format, dependency, public schema,
configuration, environment variable or CLI option changed. Two shared test suites,
the existing parity workflow, the standard library contract, changelog and parity
matrix carry the regression evidence.
The matrix adds six direct native tests and eighteen shared-source CLI tests.

Dictionary helper key acceptance is now consistently string-or-integer for
`get`, `get_default`, `has_key`, and dictionary `remove`. This preserves the
previously working specialized-map calls and extends ordinary/fixed-map support;
the checker uses a precise union rather than weakening its validation to any type.
`contains` retains its string-key-only dictionary contract.

## Baseline and verification

The unchanged full release gate passed before source edits. The new initial
matrix then failed all three direct representation tests and eight of twelve
CLI parity tests; four controls passed. Evidence is retained locally under
`.audit-evidence/native-parity/`: `baseline.log`, `reproductions.log`,
`individual-reproductions.json`, and the explicit null-read, unique and key probes.
A copy of the original executable was preserved before a successful modified
build, allowing later-discovered defects to be checked on the original source.
That temporary executable copy was removed during build-cache cleanup after
the original-source reproduction receipts had been saved.
The separate JSON/TOML/YAML and schema probes confirmed that failures masked by
earlier assertions were independently reproducible.
The integer-helper probe additionally recorded both the old checker warnings
and the VM/interpreter result mismatch in `numeric-helper-baseline.json`.
The expanded schema matrix checks that normalizing a specialized dictionary
retains the budget for siblings already queued in a nested comparison.

The huge-key test is a regression-only safety check of the same small-gap defect;
we intentionally did not attempt an allocation proportional to i64::MAX on the
baseline. No runtime speed, memory benchmark, token savings or provider-billing
claim is made. Code-supported resource improvement: sparse insertion now stores
assigned entries instead of allocating through the largest integer key. For the
measured small-gap probe, visible entries change from three to the correct one.
Merge and update now share one dispatch implementation; existing fast paths remain.

Commands (Cargo commands use `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0`):

- `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full` — baseline passed.
- `cargo test --test native_representation_matrix --test native_representation_parity --no-fail-fast` — baseline reproductions failed as recorded above; final result recorded below.

Final-source receipts (also use `CARGO_BUILD_JOBS=2` for Cargo builds):

| Command | Result | Local evidence |
| --- | --- | --- |
| `cargo test --test native_representation_matrix --test native_representation_parity --no-fail-fast` | Passed: 6 direct native tests and 18 CLI parity tests | `matrix-receipt.log` |
| `KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full` | Passed: hygiene, formatting, all-target/all-feature Clippy, full tests, focused security/package/parity/socket suites, dual/interpreter fixtures and dependency audit | `full-final.log` |
| `target/debug/kujo test --runtime vm` | Passed: 150 fixtures, 11 explicit skips | `vm-fixtures.log` |
| `cargo test --no-default-features --features runtime-archive,runtime-image --test vm_interpreter_parity_surfaces --test native_representation_matrix --test native_representation_parity` | Passed: 123 existing parity tests, 6 native matrix tests and 17 CLI matrix tests; the network-feature AI construction test is excluded in this build | `no-default-parity.log` |
| `cargo build` | Passed; restored the default-feature development binary | `default-build.log` |
| `cargo test --test readme_contracts --test architecture_docs_contract --test docs_policy_consistency_contract` | Passed: 3 documentation contract tests after roadmap updates | `final-doc-contracts.log` |
| `bash scripts/release_candidate_gate.sh --roadmap-only` | Passed after marking item 2 complete and item 3 ready | `roadmap-gate.log` |
| `cargo fmt --check` and `git diff --check` | Passed final formatting and whitespace checks | Terminal receipts |

The main `cargo test` aggregate rises from 3,017 to 3,041 passing test receipts,
with zero failures and 18 existing ignores in both runs. These totals include
one nested subprocess test receipt in each run. Dual and interpreter fixture
sweeps each pass 150 fixtures with 11 explicit skips; dual uses zero interpreter
fallbacks. Optional `cargo-deny` remains unavailable, as on the baseline.
`cargo audit --deny warnings --ignore RUSTSEC-2025-0141` passes using the gate's
existing documented exception for the unmaintained build-only bincode dependency.
Benchmark smoke is not enabled: optimized performance measurement is roadmap
item 3, and this campaign makes no speed claim.

One final-source gate attempt stopped in the large-repository docgen test with
`No space left on device (os error 28)`. This was an environmental failure:
after removing regenerable package build artifacts and release dependency
intermediates, the unchanged test passed. Source, dependency lockfiles, release
executables and reproduction receipts were preserved. The subsequent gate uses
`CARGO_BUILD_JOBS=2` to bound concurrent linker space; test concurrency, assertions
and timeouts are unchanged. Future build-time benchmarks must establish a fresh,
explicit cache baseline rather than compare across this cleanup.

The rerun also caught a generated TODO inventory line reference moved by the
type-checker signature change. `bash scripts/generate_v1_code_todo_triage.sh
--strict` regenerated the Markdown/CSV pair: one line reference changed from
5154 to 5165, the generation date advanced, and all 25 classifications remained
unchanged. The freshness assertion was retained.

## Security, compatibility and remaining scope

Reviewed dictionary input, native dispatch, serialization/redaction, allocation,
immutable bindings, schema traversal budgets and restricted filesystem access.
No capability is broadened. Sparse storage removes a key-proportional allocation
path; Kujo remains a local-code runtime, not a sandbox. Existing error exit class
4, capability ordering and secret redaction remain tested. Corrected missing-key
messages describe absent string keys consistently across both engines; VM and
interpreter diagnostic wrapper codes intentionally remain distinct.

Public API names and arities, file formats, schemas, config and environment variables
are unchanged. The three checker signatures for dictionary key lookup now
precisely accept string-or-integer keys, matching the corrected runtime contract.
Programs relying on phantom keys, collapsed string keys, omitted
nulls, duplicate equal dictionaries or silent empty merges observe corrected
behavior. Previously rejected integer storage serializes under existing formats;
previously valid ordinary/fixed serialization remains compatible. No release was
published during this work; stable remains 1.8.1.

Cross-repository follow-ups: none required. Existing platform/provider matrices
remain the evidence for Windows and remote databases; this local macOS campaign
does not substitute for their platform runs. Intentional ordering differences,
CSV row restrictions, TOML null conversion and experimental JIT scope are listed
above. No speculative API expansion or cosmetic cleanup is included.
