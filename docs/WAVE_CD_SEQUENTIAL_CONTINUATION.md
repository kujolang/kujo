# Wave C/D: bounded sequential continuation

2026-09-29. Experimental local trusted-host scope; Kujo 1.6.0 runtime and stable
contracts are unchanged. This follows the historical [one-effect admission
proof](WAVE_CD_ONE_EFFECT_ADMISSION.md). Validation passed: 80 sequential scenarios, four-language lifecycle vectors,
the complete Dispatch gate, Workcell canonical and adapter checks, and Kujo
documentation/locked build checks. See the [Dispatch evidence receipt](https://github.com/kujolang/dispatch/blob/main/docs/evidence/sequential-effects/validation.json).

Dispatch owns an append-only selection lifecycle. A still-unconsumed attempt may
receive independently renewed evidence through an explicit rebind, preserving
its immutable original selection and attempt identity. Explicit cancellation
records intentional abandonment only before any durable admission claim exists.
Neither operation can revive a consumed or possibly-mutated attempt.

After C crosses final admission and independent verification establishes its
completion, a separate fresh assessment may support selection of D. D receives
its own selection, attempt, evidence basis, final revalidation, and permanent
one-use consumption. There is no loop that executes the remainder. Historical
parent results remain untouched, complete effects cannot be selected again, and
uncertain prefixes block progression. Completing the effect list does not itself
finalize the historical parent or advance other workflow steps.

The same generic Dispatch control runs against SQLite unique-key transactions
and Workcell Git ref/marker CAS. SQLite owns row truth; Workcell owns Git truth.
Installed family adapters map exact identities, observe one effect, and execute
one checked mutation. Participants provide metadata and observations only. No
new participant wire contract, language syntax, or runtime policy is introduced.
New host lifecycle records have additive independent portable commitment vectors;
old alpha/beta and participant vectors retain their original bytes.

The existing persisted negotiation commits one profile per run. The two families
are therefore exercised in separate homogeneous runs. A mixed SQLite/Git run
would require an explicit composite authority design; one profile cannot safely
stand in for another. This is a contract boundary, not a claimed mixed-sink proof.

Remaining architecture boundaries include general scheduling, parallel effects,
DAGs, compensation, remote authenticated trust, hostile participants, multi-host
authority, distributed or machine-loss recovery, cross-sink atomicity, stable or
public SDKs, protocol freeze, A2A, exactly-once execution, and universal rollback.

Observation is not authority. Renewal is not authority. Rebinding is not admission.
Selection of C is not selection of D. Admission is not proof of mutation.
Consumed attempts cannot be revived. Sink truth remains sink-specific.

The recommended next architecture phase is broader
durable recovery and operator reconciliation on a retained trusted host. Current
orphan-claim and journal/checkpoint divergence correctly fail closed; an explicit
recovery contract should make those states inspectable and reconcilable without
reviving consumed attempts. More local happy-path variants have diminishing
value. Remote authentication is a separate later threat-model decision and would
not repair these recovery semantics.
