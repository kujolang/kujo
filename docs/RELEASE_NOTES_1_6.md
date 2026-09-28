# Kujo 1.6.0 release candidate

Status: **candidate, unpublished**. Intended tag: `v1.6.0`; no tag or publication
is authorized by this preparation. Published native/npm 1.5.0 remains the public
install default until separately authorized publication. Source and npm manifests
in this candidate are 1.6.0; their presence is not evidence of registry publication.

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

## Candidate evidence and release authorization

`release/kujo-1.6.0-rc.json` defines source/package identity and archive names.
`docs/evidence/kujo-1.6-rc/manifest.json` is the subsequent receipt identifying the
exact tested source commit, artifact hashes and downstream revisions. The receipt
commit is not the artifact source or tag target: this avoids a self-referential
commit hash. Tag only the manifest's tested source, never an untested later HEAD.

Only macos-x64 is built/tested locally. Linux x64/arm64, macOS arm64 and Windows
x64 require their normal platform gates before a full multi-platform publication
claim. No hosted CI or other-platform certification is inferred from local success.

Authorization remains required to create/push v1.6.0, create the GitHub release,
upload reviewed native/source archives and checksums, or publish runtime npm or
crate artifacts. Participant SDK publication is separate and remains unauthorized.
