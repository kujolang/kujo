# Wave F sourced resources and bounded Eval budgets

Experimental retained trusted-host Dispatch work. The existing graph lock, journal,
static DAG, attempt accounting and independent terminal authority remain the control
model. Kujo runtime and language semantics are unchanged.

Dispatch adds opt-in, immutable Eval dispatch ceilings and per-node bounded Eval
attempts. Reservations hold distinct program and Eval units; durable dispatch consumes
them before work. A plan is informational and cannot run nodes. Locked application
revalidates the exact active path, inputs, installed authority and remaining capacity.
Inactive branches reserve nothing; subgraph members count once in each applicable
dimension. Receipts and human decisions do not become execution attempts.

An installed evaluator reads exact producer/result inputs under a one-use retained
claim. A conclusive terminated infrastructure failure can permit an explicit second
Eval attempt over the same inputs. The underlying program is not repeated. Missing
acknowledgements remain consumed and unknown; retained exact response evidence can be
read back separately. Eval resolution, node finalization and graph finalization remain
distinct boundaries. Exhaustion blocks new work, not truthful result publication.

Existing runtime-measurements/v1 artifacts supply process-scoped observations.
Existing Agents SDK context-ledger/v1 artifacts supply provider-reported usage only
when their source says provider; estimates remain separately labelled. Missing usage,
CPU/RSS or monetary cost remains unknown. Source artifacts are retained by exact digest
and attributed by installed trusted-host adapters to consumed attempts. Watchdog
observes and RunLedger reports; neither receives Dispatch budget authority.

No hard token, wall-time or currency ceiling is claimed: a post-hoc reading does not
supply a safe worst-case reservation. The closed required-provider-usage policy can
block new work when previous consumed attempts lack complete sourced usage. No
inferred pricing, default currency or zero-for-unknown accounting is added.

See Dispatch `docs/contracts/resources/{decision,protocol,crash-matrix}.md` and its
resource-accounting validation evidence for exact source pins and completed proofs.
No participant wire or historical portable commitments change. Concurrent unrelated
Kujo runtime/editor edits are deliberately excluded from this documentation branch.

Remaining boundaries include hard numeric resource enforcement, dynamic topology,
general retries, distributed reservations, remote authenticated trust, machine-loss
recovery, mixed profiles, compensation, exactly-once, universal rollback, public SDKs
and new language syntax. Recommend bounded human adoption/operator usability next:
connect one installed workflow to inspect/plan/apply and exact sourced evidence.
Use that experience to identify remaining Wave E lineage gaps before adding loops,
remote trust or more resource counters.
