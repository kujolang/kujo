# Wave F execution evidence — 2026-10-07

Companion to [the source audit, architecture map, failure matrix and plan](WAVE_F_EXECUTION_AUDIT_2026_10.md).
The audit was committed before implementation (`ccaab63`); its subsequent design
clarifications are `6238951`. The selected Dispatch correction is
`93b747acca04fe80bf66b8c23fa3e69be43fad60`, based on main
`e1bbc21166022b00fb21ed000cc4f3cbb26e7e07`.

## Result and contract scope

Only **Kujo documentation** and **Dispatch settlement code, tests and documentation**
change. A settled static subgraph can be explicitly finalized while unrelated
graph nodes have held program/Eval capacity or uncertain Eval dispatch. Its own
held or uncertain members still block settlement, including optional members.
Whole-graph finalization still includes every node. Global ceilings are still
validated; the correction neither releases capacity nor changes admission.

No public CLI, v1 schema, participant encoding, controller feature, immutable
record format or portable fixture changes. Existing installed controllers remain
source-bound; this is not automatic migration of retained runs. No Kujo runtime
source changes belong to this tranche. Watchdog's pre-existing configuration edit and Agents SDK's unrelated
maintenance files are preserved. RunLedger was fast-forwarded to fetched main;
AI SDK's source-equivalent older checkout was left unchanged during verification.
Independent Workcell and Watchdog commits landed during the session; their exact
final heads and unchanged graph-evidence/measurement-adapter paths are recorded in
the audit and validation receipt. They are not included in this tranche's authored
commit list. Installed operator dependencies remained pinned.

## New adversarial proof

`dispatch/tests/subgraph_settlement_scope.mjs` adds five proof groups to the
canonical release gate. Each controller operation uses a fresh Kujo process.

1. Unrelated held program capacity permits group settlement, while graph sealing,
   stale/duplicate candidates and accidental dispatch remain rejected.
2. Unrelated held Eval capacity permits group settlement; graph sealing is blocked.
3. SIGKILL after unrelated durable Eval dispatch permits only group settlement;
   redelivery and refund remain rejected and the consumed ledger is unchanged.
4. Required human abort cannot hide a held optional program/Eval member or an
   uncertain consumed Eval member. Explicit safe release permits the failed outcome.
5. Required human cancellation has the same settlement obligations; cancellation
   does not erase optional member uncertainty or manufacture a result.

Three SIGKILL scenarios run per invocation. Assertions also check unchanged
attempt history, absent worker delivery, preserved consumption and absence of
child execution admission. Unchanged main reproduces the semantic failure with
both the retained local runtime and released 1.8.0. An initial temporary-worktree
import-root setup error was corrected by using a sibling worktree; it is not
counted as the reproduction. No existing tests or historical fixture bytes were
weakened or rewritten.

## Verification ledger

Dispatch verification completed: **112/112 baseline suites** and **113/113
corrected suites**, including 24 shards with 101 passing source contracts in each.
Both command-surface smokes passed and both bounded workloads completed 3/3 runs.
There are zero remaining failing Dispatch suites. The first 1.8.0 invocation passed
34 suites, then exited 1 because the fresh worktree lacked `.operator-deps`.
`bash scripts/install_operator_dependencies.sh` installed the declared Workcell
and Eval pins. Verification resumed with an exact copy of the canonical commands
from `operator_rehearsal` onward, retaining the previous passed suites and original
failure log. No assertion, source or historical fixture was changed for this setup
failure. After 13 more passing suites, Python isolation found a missing offline
wheelhouse. The unchanged baseline wheels were copied, and the isolation test
verified their hashes against the committed lockfile. A second exact continuation
resumed at `python_isolation`, retaining 47 completed suites. Both setup failures
remain in the logs. This is resumed canonical coverage, not an uninterrupted
successful run.

| Executed command / scope | Result established |
| --- | --- |
| Dispatch baseline `DISPATCH_OFFLINE_FIXTURE=true KUJO_BIN=<retained-1.7.0> bash scripts/run_release_gate.sh` | Exit 0; 88 focused suites + 24 shards = 112; 101 source contracts, 0 failed; 3/3 workloads |
| Corrected Dispatch, same canonical gate with released 1.8.0 and exact continuations described above | Final continuation exit 0; 89 focused suites + 24 shards = 113; 101 source contracts, 0 failed; 3/3 workloads; 2 resolved setup failures retained |
| Kujo `cargo fmt --check`, before and after | Passed |
| Kujo `cargo test --test readme_contracts` | 1 passed |
| Kujo `cargo test --locked --test architecture_docs_contract --test workflow_control_contracts` | 12 passed; 2 existing ignored tests require generated handoff fixtures |
| Kujo `cargo test --locked --test readme_contracts --test architecture_docs_contract` after docs changes | 2 passed; repeated contracts, not 2 additional unique tests |
| New Dispatch scope regression with retained 1.7.0 | 5 proof groups passed; 3 SIGKILLs |
| New Dispatch scope regression with released 1.8.0 | 5 proof groups passed; 3 SIGKILLs |
| Workcell `kujo run tests/retained_workspace_test.kujo` | 7 passed, 0 failed |
| Eval `kujo test-run tests/control_contract_tests.kujo` | 3 passed, 0 failed |
| Agents SDK fetched main `KUJO_MODULE_PATH=<audit-worktree> kujo test-run tests/context_ledger_tests.kujo` | 6 passed, 0 failed |
| RunLedger `KUJO=<runtime> node tests/runtime_measurement_reference.cjs` | Passed |
| MCP fetched main `MCP_ROOT=<audit-worktree> KUJO_BIN=<released-1.8.0> node tests/mcp_ability_integration.mjs`, then `--lost-response` | Both passed; current transport main independently verified |
| Watchdog pinned 1.6.0 `KUJO_BIN=<runtime> node tests/runtime_measurements_adapter_check.js --integration` | 76 input cases plus HTTP/RunLedger integration passed |
| Watchdog retained 1.7.0, same command | Failed at the same optional-counter negative test |
| Watchdog released 1.8.0, same command | Failed: negative test treats optional `bytecode_compile_wall_ns` as required |
| New JS syntax, gate shell syntax, changed-tree whitespace | Passed |

The Kujo tests represent 13 unique passing contracts, zero failures and two existing
ignored cases. The full Kujo runtime test suite was not run: this repository's
changes are documentation only. Standalone full gates were not run for unchanged
ecosystem repositories; targeted owner tests and Dispatch's cross-repository
canonical integration gate provide the stated coverage. Inspection is not a test
result. An initial Agents SDK `run` invocation only defined tests and is excluded;
the subsequent `test-run` executed all six.

## Crash and compatibility coverage

These are existing suites rerun against the correction, not additional new-test
counts. Their logs retain individual proof names and fresh-process fixture roots.

| Boundary | Executed evidence |
| --- | --- |
| Binding, dispatch, admission, partial composition and joins | `node_composition` (19 proofs) and `node_recovery` (5): independent finalized producers, required join inputs, crashes around binding/dispatch/admission, competing fresh controllers and exact orphan binding/terminal repair. |
| Program attempts and capacity | `graph_attempt_recovery`, `resource_recovery`: last-unit contention, immutable reservation publication, explicit pre-dispatch release, consumed dispatch before child load/admission, no refund or retry from silence. |
| Group and graph finalization | `static_policy_recovery`, `heterogeneous_recovery` (8): concurrent finalizers, before/after terminal and immutable-index gaps, exact group receipt repair without implied parent execution. |
| Eval and human review | `resource_recovery`, `heterogeneous_recovery`, `operator_rehearsal`: lost Eval reply with exact readback, unknown consumed dispatch, real terminal orphan repair, stale/wrong review decisions, approval distinct from Eval dispatch. |
| External effects and bounded effect cancellation | `sequential_effects` (80): SQLite/Git crashes after admission/mutation/observation, before/after selection/rebind/cancel, cancellation/admission races, unknown prefix and orphan claim blocking, actual pre-lifecycle reader rejection. These do not prove graph-wide cancellation propagation. |
| Parent finalization | `parent_finalization` (31): SIGKILL before decision and after durable terminal, concurrent finalizers, orphan recovery, required descendant/authority/effect/output/Eval/preservation checks and exact Workcell bytes. |

The new scope suite adds three killed Eval-dispatch scenarios and held-member
failure/cancellation combinations. General nested graph failure/cancellation is
unsupported and therefore not claimed as executable recovery coverage. Historical
portable commitments, participant parity and negotiation readers are checked by
the canonical gate; none of their fixture bytes were rewritten.

## Runtime and measurement identity

The existing local `kujo/target/release/kujo` reports **1.7.0**, despite the source
checkout being post-1.8.0. Its SHA-256 is
`2f9242613c497736f77c20fdb669a18b7c815e489e8a2e8cf5047ee75fd2e3e0`.
The final gate uses the checksum-verified released macOS x64 **1.8.0** binary,
SHA-256 `979d49503d71eb87e5696aea844a51e7129e23c89e36abc8e4d4378bc78985b5`.
The downloaded archive hash is
`af5177b9f0902bf63d28056793a6b0e85533faf1f9813d11b8bd4b4517b0f9b5`.
No result from the ambient older PATH executable is attributed to 1.8.0.

Five fresh-process settled two-node group assessments per variant used the same
1.7.0 binary: baseline median 1278.939 ms; corrected median 1000.847 ms. Concurrent
machine load was uncontrolled and runs were not interleaved. This is a bounded,
startup-inclusive cost characterization, **not a measured speedup claim**. No
optimization was introduced. Admission, persistence and artifact protocols are
unchanged; no general throughput or recovery latency guarantee is established.

## Evidence retention and unresolved limits

Raw command logs, source pins, semantic reproduction, measurement samples and
canonical suite logs are retained in
`/Users/robertdevore/2026/Kujolang/validation/wave-f-20261007/`.
Dispatch's `docs/evidence/wave-f-settlement/validation.json` contains exact commands,
runtime hashes, invocation exit codes, suite names and workload receipts. Raw logs
have a `SHA256SUMS` manifest. Publication commit IDs and memory retrieval results are recorded in the
session handoff rather than self-referentially embedded into their own commits.

The independent Watchdog test defect remains open as SignalBox capture
`cap_53c47b3e-265d-4c3f-9de3-fca151d836be`. Exact-ID and concept retrieval passed;
zero Signals and zero duplicates. Completed work, routine verification and the
requested future architecture plan were not admitted as captures.

Wave F remains experimental. This work establishes no exactly-once external
effects, rollback, automatic unknown-effect replay, general cancellation
propagation, dynamic topology, nested graph lifecycle, universal capability
attestation, hard token/time/currency ceilings, remote trust, hostile-storage
recovery, machine-loss restore, distributed execution or production readiness.
