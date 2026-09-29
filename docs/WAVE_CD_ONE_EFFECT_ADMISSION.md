# Wave C/D: one selected effect under Dispatch authority

2026-09-29. Experimental, unpublished. Kujo runtime 1.6.0 and stable v1 contracts
are unchanged. This follows the [read-only effect-set proof](WAVE_CD_EFFECT_SET_PROOF.md).

Dispatch now exposes separate operator calls to select, admit and inspect one
specific not-started SQLite effect after a verified complete prefix. Selection
uses the existing advisory run lock, guarded authoritative state, exact persisted
required/deny configuration, control journal and immutable artifact primitives.
Admission uses a durable exclusive one-use claim and repeats current authority,
registration, environment, preservation, plan/result/set, state/journal and
freshness checks inside the SQLite writer transaction before inserting.

The assessor remains read-only. A/B completion is independently verified; only C
is passed to the mutation adapter; D receives no authority. Unknown prefix blocks
selection. The parent result is not rewritten and the parent action cannot replay.
A required feature prevents the actual previous controller from ignoring the new
state. Participants and SDKs still provide knowledge only.

The proof uses actual paused workflows, real SQLite effects, independent readback,
two fresh controllers released against the same durable selection, and SIGKILL at
all five boundaries. A mutation-audit trigger distinguishes attempts from final
rows so sink deduplication cannot conceal duplicate admission. Crash before
selection leaves no attempt. Crash after selection retains the same attempt.
Crash after admission retains uncertainty even if no effect occurred. Lost reply
after mutation is independently observed without rerunning. Persisted evidence
survives controller death. A consumed attempt is never automatically reclaimed.

See Dispatch's [production admission model, authority table, crash matrix and
TOCTOU review](https://github.com/kujolang/dispatch/blob/main/docs/contracts/effect-set/one-effect-admission.md)
and [verification receipt](https://github.com/kujolang/dispatch/blob/main/docs/evidence/effect-continuation/validation.json)
for exact source commits, commands, counts, warnings and retained transcripts.

The scope is one selected attempt per surviving local run, using the SQLite
family. It does not execute all remaining effects. Evidence renewal does not
silently rebind an existing immutable selection. At this historical milestone, explicit reconciliation for a stale selected
attempt was the next policy question; the subsequent lifecycle tranche below
proves that operation within its separately feature-gated API.

Generalized production scheduling, remote authenticated trust, malicious
participant verification, remote renewal, multi-host authority, compensation,
cross-sink atomicity, stable/public SDKs, protocol freeze, A2A, machine-loss
recovery, exactly-once and universal rollback remain unsolved.

## Historical next task — addressed by the sequential lifecycle tranche

Start from current Dispatch and Kujo main. Read the one-effect admission contract
and validation receipt. Design and prove an explicit reconciliation operation for
an **unconsumed** selected attempt whose evidence expired: append newly verified
evidence without rewriting the original selection, preserve the same logical
attempt, and bind the replacement current eligibility under the existing run lock.
Never clear a consumed claim, admit an unknown prefix, replay the parent or grant
the remainder. Race renewal against admission in fresh processes, kill between
publication stages, preserve old readers' fail-closed behavior and verify the
existing complete canonical gate. Do not expand into generalized scheduling.

Subsequent work: [bounded sequential continuation](WAVE_CD_SEQUENTIAL_CONTINUATION.md)
adds the verified separate lifecycle API. This document retains the original
one-effect proof's scope and evidence; use the follow-up for current architecture
state and the next durable-recovery phase.
