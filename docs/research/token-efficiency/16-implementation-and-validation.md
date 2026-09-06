# Token-efficiency implementation and validation

Implemented 2026-09-06 from the current checkout, preserving the original audit
as historical evidence. The runtime remains Kujo 1.3.1; no syntax or model-provider
transport changes were introduced.

## Current-code evidence and scope

Starting Kujo HEAD was `114a5a9` on `main`, with a clean working tree.
Agents SDK started at `f5e7312` on `main`. Its four pre-existing untracked paths
were preserved: `docs/MAINTENANCE_AGENT.md`, `maintenance_agent.kujo`,
`src/maintenance/`, and `tests/maintenance_agent_tests.kujo`.

Inspected sibling HEADs: AI SDK `71bad14`, Dispatch `83eea7d`, Scout `6824903`,
Scent `6e72cd0`, Spec `1211f37`, kujo-agents `6298807`, kujo-skills `0d41ab4`.
The existing runner passes parent output to a child; it does not automatically
replay all parent history. Session restoration already separates run state from
model messages. Those defaults remain unchanged. The shared implementation
belongs at the Agents SDK adapter boundary, with ArtifactStore/SessionStore
extensions rather than new independent stores. Other siblings were inspected
but not modified.

## Delivered contracts

The canonical implementation is in
[Agents SDK context contracts](https://github.com/kujolang/agents-sdk/blob/e0be75b/docs/CONTEXT_CONTRACTS.md),
with modules under `src/agents/context/` and exported package entrypoints.

| Stage | Delivered behavior | Rollback |
| --- | --- | --- |
| Ledger | Hashes and exact UTF-8 bytes/Unicode characters for adapter messages, schemas and output; explicit attribution, retries, classifications and provider/heuristic separation; chat and stream sink delivery verified | Omit `context_observer` |
| Manifest | Versioned context metadata on existing artifacts, provenance, selected/available references, hash/size checks and stale-source rejection | Use original ArtifactStore interfaces |
| Handoff | Bounded typed envelope and evidence references; full parent/child evidence retained; existing loop/depth and approval/cancellation behavior preserved | Omit `context_handoff` |
| Resume | Compact state plus separate full evidence in existing stores; verified current source hashes and referenced outcomes; pre-dispatch rejection of stale state | Omit context resume/state options |
| Scoped context | Executable tool-registry filtering, schema IDs/catalog/detail expansion, full-schema fallback, summary plus artifact outputs, explicit symbol/import/dependency source selection | Omit context tool scope |
| Skills | Selected cores and mandatory security references; explicit reference triggers; integrity checks and bounded dependency loading | Keep existing application skill loading |
| Ratchet | Versioned byte/character/estimate baselines, normalized hashes, inventory/schema/handoff/retry checks, approved-growth evidence linkage and paired offline corpus | Keep default payloads; baseline changes remain explicit review items |
| Dispatch | Runtime lifecycle/tool/skill/handoff evidence, provider/model receipts, configured role binding and required-dispatch checks | Omit context execution options |

No provider discovery protocol is fabricated. On-demand schemas require an
application that actually implements detail fetch and model continuation;
complete schemas are the default. Structural repository indexes are explicit
adapter views of existing producer outputs, not a new semantic search system.
Catalogs, current provenance and capability ceilings must come from trusted
application configuration. Hashes prove integrity against trusted references,
not authenticity of attacker-supplied catalogs.

The ledger measures the adapter envelope, not final provider wire serialization.
It does not apportion aggregate provider usage into invented component counts.
Missing provider tokens, cost and latency remain null. The seven paired scenarios
are synthetic contract fixtures, not measured production model performance.
No production savings percentage or task-quality improvement is claimed.

## Verification evidence

- Kujo: `cargo build --bin kujo`, `cargo fmt --check`, and `cargo check` passed.
- All unit/integration targets were covered across the broad run and targeted
  continuation: 2,537 passing Rust test executions across 70 targets; 14 ignored.
  All `tests/*.rs` targets have a passing final result. Doc tests also completed
  with one documented ignored test and no failures.
- Broad execution used `RAYON_NUM_THREADS=2 TOKIO_WORKER_THREADS=2` and
  `--test-threads=1` after the host temporarily rejected subprocess creation.
- `kujo test --runtime vm`: 150/150 runnable fixtures passed; six skipped.
- `kujo test --runtime dual`: 150/150 passed; six skipped; zero interpreter fallback.
- The documented AI enterprise showcase passed with strict committed replay
  cassettes (`KUJO_AI_REPLAY_MODE=strict`), without live provider credentials.
- Agents SDK clean-checkout verification: all 39 offline checks passed. Context
  evaluation includes 20 paired repetitions of seven scenarios, current and
  optimized source execution under VM/interpreter, and adversarial context gates.
- The token ratchet passed with no warnings or regressions. Baseline, approval
  and evaluation JSON are committed under `tests/fixtures/context/`.
- Real observer persistence after hook composition is asserted outside the
  fail-open sink; sink failure cannot make the regression test pass. Typed child
  evidence, repeated resume, wrong-role dispatch rejection and unselected-tool
  denial are exercised through the actual runner.
- Existing default runner, result/event, approval, Ability, retrieval, session,
  memory, streaming and telemetry contracts remain covered by the offline gate.

Core verification exposed three pre-existing maintenance issues, repaired in
small commits: shared environment locking for the secret-key AI test (`79e02a4`),
generated TODO line references (`11125b7`, regenerated with the canonical script),
and the scope test's stale Linux ARM64 wording (`0c91066`). These do not change
runtime behavior. The generated TODO inventory changes only five line references.

## Existing unresolved finding

The original untracked maintenance test has two failing assertions and cannot be
used as a clean-checkout release gate. Its files were neither changed nor committed.
The tracked suite was verified in an isolated detached worktree.

Separately, the existing `redact_tool_io` helper in
`agents-sdk/src/agents/security/approval.kujo` reproduces loss of ordinary evidence:

```kujo
from src.agents.security.approval import redact_tool_io
print(to_json(redact_tool_io({
    "api_key": "fixture-key",
    "source": "retain exact evidence",
    "nested": {"value": "retain nested evidence"}
}, {})))
```

On the tested runtime, `api_key` is masked but `source` and `nested` become null.
Root cause remains unconfirmed. The new context evidence traversal consumes the
existing policy definitions while independently retaining non-sensitive values;
a regression test asserts that behavior. The older helper remains unchanged.

SignalBox records this unresolved issue only:

- Capture: `cap_09cc07ac-d928-421b-8974-da6f95315380`.
- Signal: `sig_1ef465c6-0d34-48e4-b942-6e94a33e901d`.
- No duplicates were found. Routine implementation, successful verification,
  transient host limits and already-resolved issues were rejected as captures.
- Exact-ID and concept retrieval were verified.

## Commit sequence

Agents SDK: `89bcce6`, `9586593`, `133467a`, `c3f9ede`, `c9c1e2f`, `46b3f2b`,
`388188d`, `68d9b84`, `b5c8c90`, `e0be75b`. These cover instrumentation, manifests,
handoff/state, scoping, skills, ratchet, runner/dispatch, explicit CLI checks,
and real observer delivery/redacted evidence respectively.

Core runtime changes are limited to the validation maintenance commits above
and this implementation record. The audit files numbered 00–15 remain unchanged.
