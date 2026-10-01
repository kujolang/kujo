# Wave F operator-readiness rehearsal

Dispatch's next step is an operator exercise, not another execution architecture
tranche. The bounded installed batch-summary host uses existing graph, review,
resource, effect, finalization and retained-recovery APIs. It adds no Kujo syntax,
participant protocol or scheduler.

The workflow is source JSON → typed producer summary → exact human review → Eval
→ pass/publication or fail/repair consumer → explicit graph terminal decision.
Program and Eval dispatch budgets remain distinct. A local OS-user decision is
revision-bound; output existence, approval and budget availability grant no effect
authority. Unknown provider usage/cost remain unknown.

The Dispatch [operator walkthrough](https://github.com/kujolang/dispatch/blob/5c8da03b59cf40ba9bd8fe5055813e8fb3caacb6/docs/operator-rehearsal.md)
and [first-hour findings](https://github.com/kujolang/dispatch/blob/5c8da03b59cf40ba9bd8fe5055813e8fb3caacb6/docs/OPERATOR_FIRST_HOUR.md)
record installation prerequisites and the tested interface. The black-box rehearsal
uses an installed package in a fresh HOME, real Workcell Git retention, a SQLite
receipt sink, a separate Eval process and actual SIGKILL timing. It exercises
wrong/stale approvals, quality failure, budget exhaustion and retained reservation
reconciliation without manually editing state. Full gate evidence remains required
before claiming the tranche verified.

The intended stop condition is explicit: after the operator proof and canonical
gates pass, freeze further local Wave F architecture expansion and return to
product/release work. General workflow authoring/usability can follow actual user
feedback. Dynamic graphs, arbitrary retries, hard token/time/currency ceilings,
remote trust, machine migration, public SDK stabilization and new node types remain
separately prioritized work, not automatic follow-on tranches.

This is experimental trusted-host static composition with surviving local evidence,
not universal production readiness. Existing one-use/consumed-work guarantees,
independent finalization and historical canonical bytes remain unchanged. No
runtime files or unrelated concurrent Kujo edits are part of this documentation
change.
