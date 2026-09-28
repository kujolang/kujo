# Kujo 1.6.0

Released 2026-09-28. Native archives and runtime npm packages cover Linux
x64/arm64, macOS x64/arm64 and Windows x64. Source/tag:
`44af277848173664f72ca85f2a1b3b98d634ecdd`.

## Runtime release scope

Kujo 1.6 preserves supported language semantics and hardens lexical captures,
closure/upvalue ownership, generator continuations, eager async/task execution,
shared promise completion, cancellation boundaries and VM/interpreter parity.
The loop/early-return optimizer defect is fixed with permanent minimized and
instruction-level regressions. Return stack checks remain strict.

Runtime/tooling work also includes database resource lifecycle fixes, gradual
checker corrections, confined directory durability primitives and opt-in bounded
numeric runtime measurements. Measurement export has measurable short-run overhead;
there is no zero-cost profiling or complete retained-heap attribution claim.
See CHANGELOG.md and the canonical concurrency, database and measurement documents.

## Separately versioned experimental ecosystem

These are companion-repository capabilities, not APIs made stable by Kujo's minor
version. Dispatch owns failure control, durable review/checkpoints, persisted
negotiation and replay/continuation admission. Watchdog is observation; RunLedger
is evidence correlation. Neither is controller authority.

Wave C `dispatch.effect-assurance/v1beta1` is experimental, opt-in, local
trusted-host, single-effect, required/deny only in its bounded beta domain. SQLite,
Git CAS and Ability profiles are validated. Existing alpha remains supported;
`kujo.execution-result/v1` is unchanged.

Wave D `kujo.interop-handoff/v1alpha1` and
`kujo.participant-sdk-conformance/v1alpha1` remain experimental alpha. Six
participant forms and two effect families have local controlled evidence. The
TypeScript/Python participant SDK packages remain alpha and unpublished; private
artifact rehearsals do not authorize public distribution or promise API stability.
A correlation match is not replay permission. Participants report completion
knowledge; live effect owners/verifiers establish effect state; Dispatch admits.

Source-blind agent adopter rehearsal passed. Human adopter usability remains
post-release validation. It was not a human pilot.

## Explicit non-claims

No exactly-once guarantee, universal rollback, general machine-loss recovery,
remote authenticated participant trust, multi-effect assurance, generic remote
attestation, hostile total-store rollback protection or stable participant SDK is
introduced. A process exit or timeout does not establish effect truth.

## Release provenance

The [release receipt](KUJO_1_6_RELEASE.md) records exact archive/package hashes,
hosted platform verification and public installation checks. Historical RC
provenance is unchanged; it records the original macOS x64 candidate and downstream
gates. The release expansion verified the other four targets separately against
the same source. Later receipt/tooling commits are not the `v1.6.0` tag target.

Participant SDK packages remain alpha, private and unpublished. No crate
publication or participant SDK publication is implied by this runtime release.
