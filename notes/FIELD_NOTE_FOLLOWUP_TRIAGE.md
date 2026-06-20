# Field Note Follow-Up Triage

Date: 2026-06-20
Status: maintained triage index for historical `notes/` follow-ups
Owner: Kujo core/release maintainers

## Purpose

Historical session notes intentionally preserve unchecked follow-up boxes as
audit evidence. Those boxes are not, by themselves, active v1 release blockers.

This index classifies note follow-ups into three statuses:

- `active`: launch-relevant work that is tracked by a maintained checklist.
- `post-v1`: valid future work that is explicitly outside v1 release sign-off.
- `archive`: historical or superseded suggestions retained for provenance only.

Use this file as the maintained index for `V1RR-P2-001`. Do not convert an old
field-note checkbox into release work unless a maintained checklist below owns
it.

## Inventory Snapshot

Command:

```bash
rg -l "^- \\[ \\]" notes -g '*.md' | sort | wc -l
```

Result on 2026-06-20: `194` note files contain unchecked checkbox follow-ups.

The count includes old phase notes, completed implementation notes, benchmark
experiments, LSP/editor experiments, package workflow drafts, and release-flight
evidence notes. The unchecked boxes remain in place so the historical session
record is not rewritten.

## Active Follow-Up Destinations

Only these historical-note themes remain active for the v1 release-readiness
track:

| Theme | Status | Maintained destination | Historical note cues |
|---|---|---|---|
| Tag-time release publication, binary assets, checksums, and smoke evidence | `active` | `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P0-002`; `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md`; `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` `V1U-OPEN-003`/`V1U-FINAL-003` | `release`, `artifact`, `tag-time`, `PREV1-REL`, `v1u-final` |
| Package registry and Kennel launch boundaries | `active` | `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P2-002` | `notes/2026-04-29_23-01_package-workflow-initial-cli.md`, `package-publish`, `Kennel`, `kennel.toml`, registry |
| Benchmark publication strategy and SSG/cross-language claims | `active` | `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P2-003`; `docs/PERFORMANCE.md`; `docs/SSG_BENCHMARK_NEXT_STEPS.md` | `bench-ssg`, `cross-language`, `SSG`, benchmark campaign, host-specific timing |
| Editor and LSP launch matrix | `active` | `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md` `V1RR-P2-004`; `docs/EDITOR_ADAPTER_BASELINES.md`; `docs/INSTALLATION_LSP_EDITORS.md`; `docs/LSP_RELIABILITY.md`; `docs/TREE_SITTER_KUJO.md` | `lsp-*`, VS Code, Cursor, Neovim, JetBrains, editor adapter |

If an old note appears to request one of these active themes, update the
maintained destination, not the old session note.

## Post-v1 Buckets

These follow-up themes are valid future work, but they are not v1 release
sign-off blockers unless a current maintainer explicitly promotes them:

| Theme | Status | Maintained destination | Notes |
|---|---|---|---|
| Optional typing precision and runtime type enforcement | `post-v1` | `docs/OPTIONAL_TYPING_DESIGN.md`; `docs/V1_SCOPE.md` | Covers destructuring inference, module existence checks, struct field type lookup, Promise unwrap typing, permissive callable fallback, runtime enforcement, and typed-JIT guarantees. |
| Runtime/JIT optimization beyond the VM-first release posture | `post-v1` | `docs/V1_SCOPE.md`; `docs/PERFORMANCE.md` | Covers historical Phase 4/Phase 7 JIT plans, automatic hot-loop JIT ideas, register allocation, inlining, guards, and typed specialization claims. |
| VM/interpreter parity and fixture mismatch burn-down beyond current release evidence | `post-v1` | `docs/VM_NO_INTERPRETER_UNIVERSALIZATION_CHECKLIST.md`; `docs/V1_0_REMAINING_NON_RELEASE_WORK_CHECKLIST.md`; `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md` | Current release readiness uses the generated inventory and checklist evidence, not stale note counts. |
| Generated artifact, TODO, unsafe, and inventory upkeep | `post-v1` | `docs/generated/V1_CODE_TODO_TRIAGE.md`; `docs/generated/UNSAFE_INVENTORY.md`; `docs/generated/VM_RUNTIME_MISMATCH_INVENTORY.md`; freshness contract tests | Regenerate and revalidate when source markers change. |
| Native API, stdlib, HTTP, filesystem, network, process, archive, and security hardening polish | `post-v1` | `docs/V1_0_UNIVERSAL_USEFULNESS_EXPANSION_CHECKLIST.md`; `docs/NATIVE_API_SECURITY_POSTURE.md`; `docs/STANDARD_LIBRARY_REFERENCE.md` | Completed v1 hardening notes remain useful evidence; future polish belongs in maintained checklists before implementation. |
| DocGen and external-repo universalization work | `post-v1` | `docs/DOCGEN_EXTERNAL_REPOS_EVALUATION_2026-05-18.md`; current docgen docs/contracts | Treat workstream notes as roadmap evidence, not release blockers. |

## Archive Buckets

Use `archive` for:

- old phase notes whose release, roadmap, or checklist item is already closed;
- completed implementation notes that keep suggested future cleanup tasks;
- local benchmark smoke notes that were already downgraded to local evidence;
- notes whose requested behavior is superseded by current docs, generated
  inventories, or passing contract tests;
- historical "next session" files such as `notes/NEXT_SESSION.md`,
  `notes/PHASE7_CHECKLIST.md`, and `notes/SESSION_NOTES_PHASE3.md`.

Archived follow-ups are not deleted. They remain searchable provenance and may
be promoted later only by adding or updating a maintained checklist row.

## Promotion Rule

To promote a historical field-note checkbox into active work:

1. Add or update a row in a maintained checklist.
2. Link the historical note as evidence.
3. Name the owner, validation commands, and completion evidence.
4. Update this index if the work creates a new active destination.

Until those steps happen, the old checkbox remains `archive` or `post-v1`.

## Validation

P2-001 evidence is recorded in
`notes/2026-06-20_v1_0_field_note_followup_triage.md` and
`notes/release_evidence/2026-06-20_p2-001/status.tsv`.
