# Wave A ecosystem completion

## Verified starting state and implementation plan

All four repositories were clean on main and equal to origin/main after fetch.
Kujo: `5d72aab4b99e7f8c01e4c208d6c97061934c7447`; Watchdog:
`4e07223fdff6195ab03fb03658b9f0edd99db580`; RunLedger:
`e0187ea353a912247eca455e984f27893e270fd0`; Dispatch:
`73e7a87fa7fc453085fdf1b7dd88e6ed88bf31a3`. All requested prior Kujo and
Dispatch commits are ancestors. Dispatch remains unchanged.

Watchdog already supplies native lifecycle normalization, canonical v2 SQLite
persistence, artifact references, nullable usage, and identity conflict/dedup
handling (`src/telemetry_native_adapter.kujo`, `src/telemetry_repository.kujo`,
`schemas/watchdog-native-event-v1.schema.json`). RunLedger already supplies
locked receipt mutation, timestamped notes and source correlation
(`src/cli.kujo`, `src/storage.kujo`, `src/record.kujo`). Neither needs another
event envelope, receipt schema, workflow state machine, or pricing model.

Before implementation, the chosen changes are:

1. Watchdog: add a Kujo-native runtime-summary adapter and compatibility mirror.
   Read at most 8192 UTF-8 bytes beneath a caller-trusted root without symlink
   traversal. Validate required v1 fields, fixed units encoded in names, bounded
   counters and nullable process measurements. Verify an expected content address
   against SHA-256 of the exact bytes. Attach one artifact reference and fixed
   numeric attributes to an existing caller-owned native execution observation.
   Do not derive identity or timestamps from runtime duration. Preserve original
   usage/cost. Ignore schema-permitted extension content rather than copy it.
2. RunLedger: add a CLI command that verifies the same content address and stores
   a fixed structured note through the existing locked mutation path. Do not add
   schema fields or repeat runtime counters. RunLedger verifies bytes/identity;
   semantic validation belongs to Watchdog. Duplicate attachments are no-ops.
3. Tests: real measured workload -> adapter -> native normalization -> canonical
   SQLite persistence -> fresh-process RunLedger CLI receipt/correlation/note.
   Test negative shape/version/unit/numeric/bounds/digest/path cases, nulls,
   idempotence and sensitive-looking workload/extension strings.
4. Optimized builds: immutable pre-measurement `cd6d2ea` and candidate source,
   identical Cargo.lock/default features/release profile/toolchain. Reuse the
   Rust process-inclusive rotated harness, discard warmup, retain all samples
   and hashes. No benchmark alongside compilation or full gates. Investigate
   path-specific overhead before proposing mitigation.
5. Run canonical gates, review privacy/cardinality/integrity/units/null behavior
   afresh, then update roadmap and record evidence. Wave B stays bounded and
   Wave C stays design-only.

Compatibility risks: JSON consumers cannot all represent the full u64 range;
reject numbers above the interoperable exact integer range rather than round.
Public schema extensions remain accepted but never become dynamic telemetry
labels. Caller context remains subject to existing Watchdog privacy policy.
Performance risk is importer-side bounded parsing/hashing, outside VM hot paths;
runtime production overhead is measured separately. Artifact roots and receipt
stores remain caller-trusted local storage, not an adversarial sandbox.

## Implementation and fresh source review

Watchdog now has `src/runtime_measurements_adapter.kujo` plus its root compatibility
mirror. It enriches native execution observations; canonical v2 intake/storage and
privacy policy remain authoritative. The importer checks the exact-byte digest,
all v1 required fields/counters, encoded units, nullable values and resource bounds.
Only 33 fixed numeric/null attributes and one digest reference are added. Unknown
schema-permitted counters/extensions are not projected. No runtime hot-path code
was changed in this task.

RunLedger's `runtime-measurement` command uses existing locked `update_record`,
notes and correlation. It checks bounded bytes, the digest and schema marker,
then retains a deterministic note. It deliberately does not duplicate Watchdog's
semantic validator or copy selected counters into unrelated token/cost fields.
`verification=exact_bytes_sha256` describes precisely what the receipt proves.
No receipt schema, status transition, workflow policy or pricing logic changed.

Fresh review checks and outcomes:

- Cardinality: fixed 30 counters plus three process measurements; reserve the
  native-added attribute slot, reject reference/attribute overflow or conflicts.
- Privacy: omit all report text, runtime-version/unsupported labels and extension
  names; no raw paths in new telemetry/notes and fixed importer error text.
  Existing caller context remains subject to existing Watchdog privacy policy.
- Integrity: hash and parse the same bounded byte snapshot; expected identity must
  match; confined reads reject symlink/path escape/nonregular files. Hash identity
  is not execution authenticity. Trusted roots/stores remain an explicit boundary.
- Numeric semantics: exact integers limited to the interoperable safe range;
  binary CPU floats may differ by a final bit on JSON roundtrip. Tests retain
  exact counters and allow two relative machine epsilons for CPU only. Unknown
  usage/cost is never converted to zero or relabeled observed.
- Dedup: repeated attachment is a no-op; conflicting reports reject. Delivery
  retries use the same normalized batch because renormalization updates observed
  time. Existing canonical identity conflict rules remain unchanged.
- Persistence: references require retained external bytes; no cross-store atomic
  transaction, artifact retention guarantee, rollback or exactly-once claim.
- Boundaries: Dispatch, Workcell, Eval and effect contracts are unchanged. No new
  telemetry envelope, lifecycle or provider policy entered core or receipts.

## Issues investigated during validation

An HTTP ingestion attempt with the prior dev binary aborted from an interpreter
worker stack overflow. The **unchanged existing** Watchdog
`tests/telemetry_v2_api_suite.js` reproduced the same overflow with immutable
pre-measurement `/tmp/kujo-next-baseline-bin`, isolating it from this patch.
With `RUST_MIN_STACK=16777216`, the full new HTTP/restart/receipt fixture passed
on the dev candidate. This is a pre-existing debug-profile limitation, not an
excuse to skip the canonical release-build gates; release results are recorded
below. No runtime stack policy was changed to hide the issue.

The RunLedger help golden initially failed because the new command changes help.
Its explicit expected text was updated and the full module/CLI/reference gate
then passed (81 module checks plus CLI integration). A CPU float equality check
also exposed a one-bit JSON roundtrip difference; the test now uses the narrow
float tolerance described above while leaving integer assertions exact.


## Completed end-to-end proof and gates

Wave A is complete as an **unreleased end-to-end foundation**. The raw proof lives
in `benchmarks/results/wave-a-release-2026-09-26/integration/`: measured runtime
report -> Watchdog adapter -> native normalization -> canonical SQLite repository
and HTTP intake -> duplicate delivery -> server exit/restart -> API retrieval ->
separate RunLedger CLI receipt/correlation/verified note. Caller timestamps bracket
the actual workload process. The workload carries a private-looking value through
a closure, generator and async result; no value appears in the report, telemetry,
new importer logs or receipt. The artifact digest is identical across stores;
unknown usage/cost remains null/absent and existing manual values stay unchanged.

The recorded report is 1231 bytes with digest
`sha256:04f0df0bbf037b0b9f84e2518b4ad939328d91ba8db9506cd1ac85d98ca72439`.
It records one capture cell, generator state and admitted task. The fixture receipt
`2026-09-27-unavailable-wave-a-fixture-001` correlates canonical Watchdog trace
`66acf206f387a0490b977825fb3727ac` and holds one note after two attachments.
These are fixture identities, not a new workflow lifecycle.

Validation passed:

- Kujo `cargo fmt --check`, `cargo check`, complete `release_gate.sh --full`:
  strict all-target/all-feature Clippy, all Rust suites including eight measurement
  behavior/schema tests, 90 security checks, nine package tests, 117 parity tests,
  and 149 dual plus 149 interpreter fixtures. Eleven documented fixture skips
  remain. Cargo audit scanned 652 dependencies against 1271 advisories, retaining
  only the existing build-only RUSTSEC-2025-0141 exception.
- Watchdog: all 67 JavaScript entrypoints in its canonical full regression loop;
  new adapter coverage has 76 input cases plus identity/timing/cardinality,
  conflicting-reference, provenance and dedup checks. The real HTTP/restart/receipt
  fixture passes with the optimized runtime and **default** worker-stack settings.
- RunLedger: full `tests/run.sh`, 81 module tests, existing CLI/concurrency coverage,
  new artifact integrity/privacy/path/dedup tests and null/manual-usage preservation.

No failing gate was suppressed. Existing optional `cargo-deny` is unavailable;
opt-in serve-socket integration and benchmark smoke retain the canonical wrapper's
standard defaults. The separate optimized campaign provides measurement evidence.
Watchdog's existing proxy benchmark reports nonstream/stream latency budgets as
advisory and did not meet those soft budgets; strictness was not changed. These
are not the runtime instrumentation results and are not claimed green budgets.
See `validation.json` and the committed Watchdog/integration logs in the evidence
folder for exact commands, results and raw-log hashes.

## Optimized overhead and mitigation boundary

[Full distributions, paired deltas, raw samples and provenance](../benchmarks/results/wave-a-release-2026-09-26/README.md)
compare immutable pre-measurement `cd6d2ea` with candidate runtime source `5d72aab`.
Same toolchain, Cargo.lock, default features, release profile, host, harness and
inputs; two discarded warmups and 21 rotated samples per mode for six workloads.
All 414 process executions preserve stdout/stderr/status. No build or test jobs
ran during the campaign.

Paired median enabled changes were +16.95% for 10k calls, +2.86% for 1k
capture/generator iterations and +8.95% for 100 async calls. Longer inputs showed
+0.66%, +0.92% and +1.43%. Disabled paired medians ranged −4.22% to +0.14%; negative
deltas are not speedup claims. Short capture and async samples show substantial
spread, and these observations do not establish a universal overhead bound.

Focused probes identify synced file export as the main observed fixed-cost
boundary: about 22.28 ms median versus 1.63 ms unbuffered export without sync.
Actual snapshot including process usage/report construction was about 12.7 µs;
uncontended saturating atomic increment about 9.5 ns; monotonic now/elapsed about
84 ns. These component probes are not an additive causal decomposition. Process
startup/parser/compiler cost remains included in the macro comparison.

A maintenance experiment can buffer the bounded report before `write_all`, retaining
exclusive creation, private permissions and `sync_all`. That targets write-call
overhead, not the sync cost. Amortize export over useful execution boundaries and
keep opt-in measurement. No measurements, counters or durability steps were removed
to improve these numbers; no runtime source changed in this task.

## Remaining boundaries and next phase

Non-blocking Wave A work: complete allocator/retained-graph attribution, long-run
production tuning, contended task/JIT-specific and additional-platform overhead,
additional consumers and artifact-store retention policy. Provider usage/cost
remains externally sourced with provenance; no provider pricing entered core.
The existing debug HTTP worker-stack issue remains open and separately reproduced.

Wave B remains Dispatch's bounded review checkpoint requiring surviving authority,
evidence and preservation. No general machine-loss restore or arbitrary checkpoint
recovery is claimed. Wave C stays design-ready and unscheduled, with no broad v2
implementation. The foundation is sufficient to begin a scoped assurance slice:
read `docs/EFFECT_CONTRACT_DIRECTION.md`, audit Dispatch's `retry_is_effect_safe`,
then prove subject-bound and independently resolved idempotency evidence with two
offline sink adapters and crash/expiry/mismatch failures before changing admission.
Preserve conservative uncertainty and the existing documented v1 behavior.

## Commit map and repository ownership

| Repository | Commit | Purpose |
| --- | --- | --- |
| Kujo | `480a087` | Starting-state audit, ownership plan and fresh review record |
| Kujo | `515d093` | Optimized harness, raw samples, component probes, hashes and integration/gate evidence |
| Watchdog | `f765e2c` | Bounded adapter, native execution projection, tests, documentation and CI integration |
| Watchdog | `5adcc26` | Actual caller timing/restart proof and published validation evidence |
| RunLedger | `ef0fa3a` | Verified runtime artifact notes through the existing receipt mutation path |
| RunLedger | `97cb607` | Preserve existing manual usage/cost; runtime prerequisite and validation evidence |

The final Kujo documentation commit records this completion state and reconciled
roadmap. Dispatch remains unchanged at `73e7a87`; no Workcell/Eval/Agents SDK or
Wave C code was modified. Watchdog and RunLedger main were pushed successfully
and clean. These are source commits, not published runtime/package releases.
