# Repository hardening: round three

## Repository and scope

- Repository: `kujolang/kujo`, branch `main`.
- Starting SHA: `711d834153cfe79b9187aa421f0691d0b3dcebfd`; clean working tree.
- Ending implementation SHA: `d496909dd3aab3b3d9020964c86d1730c9d43316`. The final report
  commit is discoverable with `git log -1 -- docs/audits/repository-hardening-round-three.md`.
- Date/platform: 2026-10-10, macOS x86_64.
- Purpose: Rust implementation of Kujo, default bytecode VM, interpreter,
  capability-aware native APIs, CLI and ecosystem tooling.
- Relevant dependencies: existing `csv`, `toml`, `regex`, `serde_json`; no manifest,
  dependency, lockfile, release-version, or sibling-repository changes.
- This is a fresh round. The preceding audits remain authoritative for their
  fixes; their ten bugs are not recounted here. This round does not assert that
  an informal estimate of ten bugs is a verified backlog.

Review covered the native serialization, schema, string/math, token/error and
file-stream paths and their existing tests and contracts. Implementation focused
on reproduced serialization/validation defects and measured unnecessary work.
The broad existing suite covers CLI output, runtime parity, generated files,
examples, networking, process lifetime and capability boundaries. This is not a
claim that every possible bug in the repository has been eliminated.

## Baseline and reproduction

The unchanged starting checkout passed **3,001 tests**, with **0 failures** and
**18 existing ignored tests**, across 107 test-result receipts. Existing warnings
came from vendored `tiny_http`; they were not suppressed or changed.

Eight new regression tests failed on the starting implementation, twice. They
covered CSV literal dictionaries, duplicate headers, dropped cells/columns,
column ordering, type-inapplicable malformed schema keywords, unvisited malformed
child schemas, empty constraint arrays and excessive TOML diagnostics. Additional
compatibility tests cover quoting/newlines/Unicode, redaction, missing cells,
short diagnostics, both runtimes, valid inapplicable constraints, annotations,
empty enum semantics, unused-schema depth and bounded regex reuse.

The proposed rejection of empty `enum` was discarded after checking the
[Draft 2020-12 validation metaschema](https://json-schema.org/draft/2020-12/meta/validation).
It has no `minItems` restriction on `enum`; the
[applicator metaschema](https://json-schema.org/draft/2020-12/meta/applicator)
requires non-empty combinator arrays. The compatibility test requires
`{valid:false}`, not a malformed-schema error. Empty `type` and combinator arrays
are malformed. No snapshots or assertions were weakened to obtain passing tests.

## Findings

| ID | Priority | Area | Finding and evidence | Action | Status |
|---|---|---|---|---|---|
| R301 | P1 | CSV/VM parity | `to_csv` rejected fixed dictionaries produced by VM literals; ordinary dictionaries worked | Shared borrowed row adapter, VM/interpreter CLI coverage | Fixed |
| R302 | P1 | CSV integrity | Duplicate headers overwrote earlier cells in `parse_csv` | Reject duplicates before reading rows, including header-only input | Fixed |
| R303 | P1 | CSV integrity | Export discarded later extra columns and replaced compound cells with empty text | Explicit errors, preserve supported scalars/missing cells/nulls/redaction | Fixed |
| R304 | P2 | Determinism | Export order followed dictionary hash iteration rather than a stable order | Lexicographic first-row columns | Fixed |
| R305 | P1 | Schema correctness | Malformed keywords were skipped when the instance had another type | Instance-independent structural validation | Fixed |
| R306 | P1 | Schema correctness | Absent properties, empty-array item schemas and unused definitions escaped checking | Bounded recursive schema inspection | Fixed |
| R307 | P2 | Schema correctness | Empty type/combinator arrays produced success or mismatch instead of malformed-schema errors | Reject empty arrays; retain valid empty enum | Fixed |
| R308 | P2 | Agent diagnostics | A small parser error echoed an entire 8 KiB TOML line | Structured byte span and bounded message for large inputs; shared Unicode preview | Fixed |
| R309 | P1 | Runtime work | Identical `items.pattern` compiled once per item | Per-call compiled-pattern reuse capped at 32 entries | Fixed, measured |
| R310 | P2 | Repeated lookup | Additional-property checks built a name list and scanned it for every field | Use existing dictionary lookup directly | Fixed; complexity improvement inferred from code |

## Changes and compatibility

### CSV integrity and determinism

`src/builtins.rs` now adapts ordinary and fixed dictionaries through one borrowed
row helper. The adapter orders names without cloning dictionaries or values.
Export retains the first-row column contract, numeric/string/boolean conversion,
secret redaction, missing/null empty cells and CSV escaping. It rejects extra
columns and unsupported cells before returning any output, instead of publishing
an apparently successful but incomplete string. Parsing rejects duplicate names.

Signatures and CSV syntax do not change. **Column ordering becomes lexicographic**.
Consumers must use headers rather than depend on previous arbitrary positions.
Callers exporting heterogeneous rows must explicitly project their desired
columns; compound cells must be serialized first. No whole-tree or streaming
serialization redesign was introduced.

### Schema structural checking and bounded reuse

`src/interpreter/native_functions/schema.rs` checks the schema independently of
instance traversal. This fixes errors hidden behind nonmatching types, absent
properties and unused definitions. Structural traversal has its own 100,000-node
budget and retains the 64-level depth ceiling. Existing instance validation and
comparison budgets remain intact. Accepted annotation values are still data.
Valid type-inapplicable constraints still do nothing.

Schema-type parsing is shared and borrows names. A per-call cache retains at most
32 compiled regexes; additional patterns are still compiled, checked and executed.
Nothing survives the call, so invalidation is automatic and there is no global
cache. The resource regression test checks reuse, capacity and invalid patterns
past capacity. Property membership uses existing lookup rather than a second name
list and linear scan. This is expected O(1) membership for ordinary dictionaries;
fixed dictionaries retain their existing linear lookup. No unmeasured timing
claim is made for that change.

Malformed schemas previously accepted by accident now return errors. Valid
schema result fields, numeric equality, local references, instance paths and
annotation behavior remain unchanged. Empty `enum` remains valid and matches
nothing. A schema document with over 100,000 structural nodes now fails explicitly
even when those nodes are not visited by the supplied instance.

### Concise TOML errors and shared diagnostics

`src/errors.rs` owns the existing 128-character Unicode-safe preview helper;
`type_ops.rs` reuses it without changing numeric diagnostics. For TOML input above
512 bytes, `builtins.rs` reads the parser's structured byte span and message,
avoiding the formatted source excerpt. Short TOML diagnostics remain byte-for-byte
compatible. Full source stays with the caller; the parser span remains actionable.

No public API signature, CLI option, exit code, JSON result schema, configuration
key, environment variable or package interface changes. Error text and stricter
rejection of malformed/lossy inputs are intentional corrections, documented in
`STANDARD_LIBRARY.md` and `CHANGELOG.md`. Existing valid Kujo programs require no
migration except callers relying on previously arbitrary CSV ordering.

## Performance and efficiency

Final-candidate paired measurements (median of 15 samples per size):

| Pattern-validated items | Original | Updated |
|---:|---:|---:|
| 100 | 47.565 ms | 1.841 ms |
| 1,000 | 534.574 ms | 12.531 ms |
| 5,000 | 2,431.305 ms | 59.198 ms |

The host was concurrently verifying the repository; ranges are preserved in the
receipts (for 5,000 items: original 2,234.669–4,961.951 ms; updated
55.437–122.195 ms). Treat these as evidence of eliminated repeated compilation,
not a stable production latency promise. Earlier trials were faster on both
versions; they remain in the raw local evidence.

The 8,197-byte malformed TOML input produced **8,304 → 64 diagnostic bytes**.
The updated native token estimator reports **16 estimated tokens** for that
64-byte message; no provider-token or billing claim is made. Existing numeric
conversion outputs remain 174/176 bytes and 44 estimated tokens. Output-size
assertions cap the TOML fixture at 1,024 bytes and check retained byte locations.

The probe is `scripts/schema_validation_bench.rs`: repeated valid `items.pattern`
checks for arrays of 100, 1,000 and 5,000 strings. Original and updated native
libraries are statically linked into otherwise identical probes. These are local
**debug-runtime** measurements, not release performance guarantees. Each trial
uses two warmups and five samples; three alternating original/updated trials
retain individual samples. No timing threshold is added to CI.

The schema correctness, cache-capacity and diagnostic-size assertions run in the
existing full `scripts/release_gate.sh` test gate, invoked by
`.github/workflows/ci-release-gate.yml`. No new CI job is needed. They provide stable regression checks without host-dependent
timing. No dependency reduction, RSS improvement, build-size reduction, or
provider billing savings is claimed.

## Security and failure boundaries

Reviewed in-memory parser, serialization and model-visible diagnostic boundaries.
CSV duplicate/lossy data now fails explicitly. Schema inspection is bounded and
validates malformed inputs without relying on instance shape; regex retention is
bounded and local. Secret redaction has regression coverage. The changes perform
no new I/O and acquire no capabilities. They do not turn Kujo into a sandbox.

File-stream no-follow, range and partial-write checks were inspected; no change
was justified in this round. No new process, network, persistent state, locking,
or retry behavior was introduced. Platform-specific Windows execution is not
claimed as tested on this macOS host.

## Cross-repository follow-ups and remaining work

No required cross-repository implementation change and no admitted unresolved
P0/P1/P2 finding from this round. No sibling repository was modified.

- P3 / not worth changing: broad style churn, serializer API redesign, and changing
  established CSV numeric/boolean parsing or TOML null behavior without a separate
  compatibility case.
- Needs more evidence: production release-build timings and peak-memory profiling
  would be needed for stronger performance claims; they are not prerequisites for
  these reproduced correctness fixes.
- Previous audit platform/isolation limitations remain as documented there.

## Verification receipt

Raw logs are under the ignored local directory
`.audit-evidence/hardening-round-three/`; compact reproduction and measurement
receipts are committed under `docs/audits/evidence/round-three/`.

Commands executed (full logs retained locally):

```sh
# Before edits: baseline.log, PASS, 3001 passed / 0 failed / 18 ignored.
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast

# Reproduction: repro-expanded-before.log, expected failure (8/8 tests).
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test serialization_schema_hardening
# Independent original-binary repeat: repro-expanded-repeat.log, same 8 failures.
.audit-evidence/hardening-round-three/repro-before-bin --nocapture

# Final focused suite: targeted-final.log, PASS, 14 tests.
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --test serialization_schema_hardening -- --nocapture

# Final complete suite: final-verified.log; result recorded below.
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 KUJO_ENABLE_SOCKET_TESTS=1 cargo test --locked --no-fail-fast

# clippy.log and check.log, both PASS.
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --locked --all-targets --all-features -- -D warnings
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked
cargo fmt --check
rustfmt --check --edition 2021 scripts/schema_validation_bench.rs scripts/diagnostic_volume_bench.rs
git diff --check

# runtime-vm.log and runtime-dual.log: PASS, 150/150 each, 11 policy skips.
target/debug/kujo test --runtime vm
target/debug/kujo test --runtime dual

# Original probe was built before edits with the same library path and source,
# output name schema-before. Final candidate command:
rustc --edition 2021 scripts/schema_validation_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-round-three/schema-final
for trial in 1 2 3; do
  .audit-evidence/hardening-round-three/schema-before >> .audit-evidence/hardening-round-three/schema-final-before-paired.txt
  .audit-evidence/hardening-round-three/schema-final >> .audit-evidence/hardening-round-three/schema-final-after-paired.txt
done
rustc --edition 2021 scripts/diagnostic_volume_bench.rs --extern kujo=target/debug/deps/libkujo-aa76783dfa658554.rlib -L dependency=target/debug/deps -o .audit-evidence/hardening-round-three/diagnostics-after
.audit-evidence/hardening-round-three/diagnostics-after
```

The first post-change full run (`final-tests.log`) caught the overly strict
empty-enum candidate in its new compatibility test. The final focused and full
runs use the corrected implementation; the superseded failure is retained as
evidence, not classified as a pre-existing failure. All baseline ignored tests
remain unchanged. Remote CI and native Windows/Linux execution are not claimed
by this local verification receipt.

Final complete-suite result: **3,017 passed, 0 failed, 18 unchanged ignored**,
across 108 result receipts. This adds 14 integration tests and one unit test (executed in both the library
and binary harnesses) to the 3,001-test baseline. Formatting, diff checks, Clippy, cargo check and both
150-test runtime modes passed. No known regression introduced by this round
remains.
