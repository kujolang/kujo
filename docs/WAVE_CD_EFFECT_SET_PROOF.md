# Wave C/D: bounded effect-set and minimum protocol proof

2026-09-29. Experimental and unreleased. Kujo 1.6.0 runtime and stable v1
contracts are unchanged. This milestone completes the bounded investigation and
proof described here, **not** Wave C or Wave D as a whole.

## Current-state audit

| Area | Verified implementation / evidence | Remaining boundary |
|---|---|---|
| Runtime | Published 1.6.0; `docs/RUNTIME_CONCURRENCY_COMPLETION.md`, current capability APIs | No new runtime policy, distributed transactions or machine recovery |
| Results | `schemas/workflow-control/execution-result-v1.schema.json` already permits multiple effects | Status alone cannot describe partial completion |
| Assurance | Dispatch `src/core/effect_assurance.kujo` and `effect_assurance_beta.kujo`; beta composes existing live resolver and v1 policy | Exactly one effect; local trusted installed verifier; existing unknown external-idempotent exception retained |
| Authority | Dispatch persisted negotiation, exact installed revisions, control journal and review checkpoint | Surviving authoritative state required; no hostile whole-store rollback protection |
| Workcell | `src/evidence/git_effect.kujo` checks live target and marker, atomic ref/CAS; preservation records constrain continuation | Retention is not suspended process state; live remote provider certification remains separate |
| Participants | Dispatch `src/core/interop_handoff.kujo`, independent TS/Python codecs and SDK recording helpers | Single-effect generic reader; no authority from correlation or knowledge labels |
| Agents SDK | `src/agents/abilities/controlled.kujo` invokes installed host admission/record callbacks | SDK cannot grant itself Dispatch authority; unrelated user maintenance-agent files untouched |
| Related tools | Eval `src/control_contracts.kujo`; Watchdog runtime measurement/native adapters; RunLedger record/note storage; Scent artifact packing; RAG retrieval; MCP framework; Leash decision transport | Evaluation, observation, provenance and transport remain separate from replay policy |

Source baselines and exact verification commands are recorded in Dispatch
`docs/evidence/effect-set/validation.json` and its audit. Current `main` was fetched
for Kujo, Dispatch, Workcell and Agents SDK. Kujo/Dispatch/Workcell matched upstream.
Agents SDK's local main is seven release commits behind fetched main `978354a`;
its controlled Ability source is unchanged across that range. It was inspected
read-only, including fetched release history, preserving user-owned untracked work.
No historical implementation branch was used as the development baseline.

Readiness documents retain dated pre-release observations. Current `ROADMAP.md`
and `docs/KUJO_1_6_RELEASE.md` own today's stable-release claim; the old 1.5/RC
wording is historical, not a reason to reopen completed release work.

## Delivered model

See Dispatch's pre-implementation
[ownership decision](https://github.com/kujolang/dispatch/blob/main/docs/contracts/effect-set/decision.md),
[formal protocol, multi-effect and freshness contract](https://github.com/kujolang/dispatch/blob/main/docs/contracts/effect-set/protocol.md),
and [remote threat model](https://github.com/kujolang/dispatch/blob/main/docs/contracts/effect-set/remote-threat-model.md).

The new Dispatch module separately assesses up to eight ordered effects using
exact parent-result bytes, an installed immutable plan, retained observation
references and independent live readback. A real local SQLite sequence is killed
after A commits and B commits before reporting. A second controller is killed
after recording its immutable observations and journal cursor. A fresh controller
retains A complete, B unknown, and C/D unstarted but blocked. B deliberately lacks
an installed independent verifier: knowledge of the fixture implementation cannot
be substituted for application verification authority.

Expiry changes current eligibility, not historical truth. Renewal appends new
artifacts and control-event references. Changed registration/configuration, target,
authority or attempt invalidates the old binding. Actual row deletion invalidates
readback. Duplicate logical keys are explicitly ambiguous; real concurrent retries
against the existing sink enforce one logical A write.

The independent Go implementation reproduces canonical wire and hashes and records
knowledge through stdin/stdout or a bounded local MCP tool. Its Go codec is one
implementation shared by two transports. It imports no ecosystem codec. TS,
Python, Go and Kujo reproduce the new seven-vector corpus; historical vectors
remain unchanged. The real lost B report is correlated through the old generic
reader using a distinct truthful single-effect child result. No old consumer is
tricked into treating the parent as a single-effect result.

## Failure and crash matrix

| Scenario / state before loss | Known effects after reload | Unknown | Evidence / expiry | Permitted next action | Prohibited action |
|---|---|---|---|---|---|
| A: A commits, B starts and loses report; participant SIGKILL, then controller SIGKILL | A independently observed committed; C/D observed not_started | B | Exact artifacts and existing journal survive | Inspect B, revalidate; retain A | Parent replay; C/D execution through this prototype |
| B: participant claims success, verifier unavailable | Original claim remains | Claimed completion | Claim has no independent confirmation | Review / independent observation | Promote claimed success into assurance |
| C: previously observed A expires | Historical A observation retained | Current A predicate | At exact deadline stale; renewal appends new refs | Fresh readback and new record | Extend old bytes, infer failure, reuse stale authority |
| D: two processes use A's logical sink key | One logical A row remains | Any unsupported external duplicate remains ambiguous | Existing SQLite unique-key enforcement; two real contenders | Retain sink dedup evidence | General exactly-once claim or count-based trust |
| Repeated logical key in one plan | None admitted | Ambiguous mapping | Plan rejects even with distinct effect IDs | Correct/review plan | Count one sink effect twice |
| Changed Git-ref predicate / deleted record / revoked credential | Historical observation retained | Current predicate | Changed-ref/revocation fixture negatives; real SQLite deletion | Revalidate under current authority | Reuse old observation |
| Expired resource / preservation / changed registration | Historical evidence retained | Current authority or environment | Expiry/ref-change negatives | Review, renew under authorized owner | Replay under stale configuration |
| E: alpha participant / beta host; new participant / historical alpha | Historical meaning retained | Unsupported combinations reject | Existing migration, frozen-reader and replay suites | Existing version-negotiated route | Relabel alpha bytes or downgrade protected state |
| All four independently observed complete | Each observation remains distinct | None under current predicates | Fresh exact readback required | Review completion | Parent replay or implicit success rewrite |
| First unstarted after complete prefix | Prefix complete; next not_started | Later eligibility unresolved | Current facts plus original not_started | Identify candidate for future locked admission | Treat assessment label as a ticket |
| Torn journal after retained checkpoint | History preserved but unreconciled | Authority continuity | Existing reconciler rejects | Operator reconciliation | Automatic resume |

Changed-ref/credential/remote-expiry cases are deterministic contract negatives,
not live remote-service tests. Git/Ability's existing real-family suites are also
rerun by the canonical Dispatch gate. A timestamp injection exercises boundary
semantics without sleeping; it does not test clock synchronization across hosts.

## Remaining boundaries

The production workflow runner does not yet schedule or resume individual effects
from the set. The new assessor is read-only; every answer requires review and
prohibits whole-parent replay. Its only positive candidate label still requires a
future existing-run-lock/authority/sink-admission integration. This is an explicit
bounded proof, not production multi-effect assurance or a beta policy extension.

Remote authenticated trust, malicious/compromised remote-host verification,
multi-host authority, remote renewal, compensation, atomicity across sinks,
stable/public participant SDKs, protocol freeze, A2A and machine-loss recovery
remain unsolved. Rust participation is evaluated, not implemented. MCP is a local
recording transport, not remote certification. No exactly-once, universal rollback
or automatic compensation is promised.

## Exact next-agent prompt

> Start from current main in kujolang/dispatch and kujolang/kujo. Read Dispatch
> docs/contracts/effect-set/{decision,protocol,remote-threat-model}.md and
> docs/audits/effect-set-proof.md, then rerun tests/effect_set_integration.mjs and
> tests/go_participant_conformance.mjs with the pinned Kujo 1.6 runtime. Implement
> the next smallest proof: bind one explicitly selected not_started effect after
> a verified complete prefix to Dispatch's existing run lock, persisted exact
> authority and one-use admission machinery. Persist the selection and a new
> effect-attempt identity before execution; reject a changed set/plan/revision,
> expired preservation/evidence or stale registration both at selection and final
> mutation. Use a real local sink and two competing fresh controllers. Kill before
> admission, after durable selection, after mutation and before reply. Prove only
> the selected effect can run, completed effects never rerun, uncertain effects
> remain blocked, and old alpha/beta/single-effect bytes and policies are unchanged.
> Do not create execution-result/v2, remote PKI, compensation or a public SDK.
> Keep the read-only assessor distinct from tickets. Record exact commits and
> canonical gates; commit/push clean changes and consolidate the new Strata delta.
