# Native field-result copying reduction

The JIT field-read helper now borrows an owned receiver directly and encodes
ordinary stored scalar fields from their references. Owned results still receive
an independent cloned value and native root. Other receivers, missing fields and
method lookup retain the canonical VM resolver. `ArgParser` is excluded from the
stored-field shortcut to preserve its legacy method precedence.

This accepts the scoped helper optimization after semantic, checksum and matched
measurement review. It does not complete the broader performance checklist.

## Measured result

Four paired process rounds alternate baseline/candidate order. Each process runs
six internally alternating rounds across five payloads and field/array access.
All 480 observations are retained: 200,000 reads each, after 100 warmup reads.
Both binaries use the same committed `native_field_read_cost.rs` source.

| Payload | Baseline median ns/read | Candidate median ns/read | Field time reduction |
| --- | ---: | ---: | ---: |
| Integer | 117.03 | 37.67 | 67.8% |
| Negative-zero float | 116.67 | 38.24 | 67.2% |
| Boolean | 116.41 | 37.37 | 67.9% |
| Null | 113.06 | 37.88 | 66.5% |
| Nested array | 183.99 | 124.78 | 32.2% |

Every paired field comparison improves. Candidate field-time ranges are below
baseline ranges for all five payloads. Array-control median ratios range from
0.9800 to 1.0025, and their raw ranges overlap. The nested-array control's small
increase remains recorded. No observations were removed or corrected for load.
The earlier unmatched baseline is retained separately and is not used for these
before/after ratios.

These are local direct-helper measurements on the loaded laptop, not whole-Kujo,
LangArena, Goose or Rust speed ratios. The experiment measures the combined helper
change; it does not attribute an exact share to receiver borrowing versus scalar
encoding. It does not establish universal zero-regression or memory/RSS results.
Production-size, platform and clean-host comparisons remain pending.

## Verification

- 1,309 library tests and 15 targeted integration tests pass. Eight existing manual
  library tests remain ignored. Exact membership of all six test binaries matches
  the recorded test output; this is not a new run of the full integration corpus.
- All 50 test-size checksums pass in each of VM, interpreter and JIT: 150 total.
  Verification checks source/config identities, runtime identity, untruncated raw
  output, exact checksum lines and successful summaries.
- The field regressions cover scalar tags, owned-result isolation, receiver
  replacement, method receivers, errors/recovery and native execution. The helper
  warmup also checks exact scalar payloads, including the sign bit of negative zero,
  and nested-array contents before measured reads.
- Formatting, diff checks, frozen source/artifact identities and raw measurement
  inventory verification pass. No runtime tests were rerun merely to capture logs.

The pre-existing `ArgParser` field/method difference remains documented in the
[parity matrix](../../docs/VM_INTERPRETER_PARITY_MATRIX.md). This optimization
preserves it; it does not choose a new cross-runtime compatibility contract.
VM and interpreter implementation files are unchanged.

## Provenance and limits

Accepted runtime commit: `fd7b764`. Baseline runtime source is `26105f4`, with
the baseline established in `61d0e4b`.
Candidate CLI SHA-256:
`45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f`.
Candidate library SHA-256:
`ffce3ed37b9cd1935ac3898ce042c354024c5b64e21148a255636dabde8bbd2a`.
Candidate helper probe SHA-256:
`4f471e93d2b2093d2562100662a8296e6119b2f001fece4fda67904ba740c957`.

The original Cargo process completed successfully; its release build took
30m51s locally. The post-test collector initially used `filter_map`, unavailable
in the installed Ruby, and was corrected before measurements. That tooling
failure did not invalidate or restart the passed runtime tests.

[Evidence bundle](native-field-abi-fix-2026-10-31.json.gz) preserves source manifests,
test output, artifact identities, all paired observations, checksum results,
verifiers and scoped acceptance. Other struct payloads, missing-method performance,
cold-start costs, generated-code CPU coverage and full-workload speed remain outside this
helper timing experiment. No public release or upstream submission is made here.
