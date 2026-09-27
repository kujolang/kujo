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
