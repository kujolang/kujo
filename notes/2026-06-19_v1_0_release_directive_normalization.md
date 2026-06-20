# V1.0 Release Directive Normalization

Date: 2026-06-19
Checklist item: V1RR-P1-007

## Decision

`UNBLOCK_V1_RELEASE` remains the standing explicit release directive for
actions that create or publish final release artifacts.

The normative policy now lives in `docs/RELEASE_PROCESS.md` section
`8.0 Explicit Release Directive`. The repeated blocker language in
`docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` is retained as audit history, but
it is no longer the source of release policy.

## Updated Surfaces

- `docs/RELEASE_PROCESS.md`: defines the explicit release directive and the
  actions that must not occur without it.
- `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`: states that tag-time sign-off
  rows stay unchecked until a real publication event occurs under the
  directive.
- `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`: annotates historical
  release-freeze loops as audit history.
- `tests/release_process_docs_contract.rs`: guards the policy section and
  required directive markers.

## Validation

Command logs and exit statuses are recorded in
`notes/release_evidence/2026-06-19_p1-007/status.tsv`.
