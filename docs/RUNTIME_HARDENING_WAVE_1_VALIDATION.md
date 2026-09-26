# Wave-1 validation ledger

Evidence dates: 2026-09-25–26. Exit status is authoritative. A failed or skipped
check is not a pass. Commands run in Kujo unless another directory is stated.

The original Agent-1 isolation used `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`.
Subsequent Kujo checks used the same job/thread/incremental limits with normal
dev/test debug profiles. No lint or latency threshold was weakened.

## Original Agent 1 isolation (f7bda5b)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 9 |
| `cargo check` | 0 | 276 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 203 |
| `cargo test` | 101 | 416 |
| `cargo test --test vm_interpreter_parity_surfaces` | 0 | 4 |
| `cargo run -- test --runtime vm` | 0 | 23 |
| `cargo run -- test --runtime dual` | 0 | 22 |
| `cargo test --test closure_capture_audit --test closure_capture_contracts --test imported_vm_callback` | 0 | 3 |
| `bash scripts/release_gate.sh --full` | 101 | 462 |

## Agent 1 correction checks (ba7f962; inventory refreshed afterward)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 14 |
| `cargo check` | 0 | 93 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 132 |
| `cargo test` | 101 | 1112 |
| `cargo test --test vm_interpreter_parity_surfaces` | 0 | 6 |
| `cargo run -- test --runtime vm` | 0 | 326 |
| `cargo run -- test --runtime dual` | 0 | 26 |
| `cargo test --test closure_capture_audit --test closure_capture_contracts --test imported_vm_callback` | 0 | 9 |

## Agent 1 full rerun after inventory refresh (761c75a)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo test` | 101 | 551 |

## Agent 1 complete no-early-exit rerun (761c75a)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo test --no-fail-fast` | 0 | 546 |

## After Agent 1 merge

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 23 |
| `cargo check` | 0 | 87 |
| `cargo test --test vm_interpreter_parity_surfaces` | 0 | 320 |
| `cargo run -- test --runtime vm` | 0 | 475 |
| `cargo run -- test --runtime dual` | 0 | 25 |

## After Agent 3 merge

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 15 |
| `cargo check` | 0 | 72 |
| `cargo test` | 0 | 1317 |
| `cargo test --test vm_interpreter_parity_surfaces` | 0 | 4 |
| `cargo run -- test --runtime vm` | 0 | 266 |
| `cargo run -- test --runtime dual` | 0 | 27 |
| `cargo test --test workflow_control_contracts` | 0 | 3 |

## Agent 2 characterization and corrected documentation contract

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo test --test runtime_semantics_characterization --test architecture_docs_contract` | 0 | 26 |

## Combined Kujo checks (first attempt)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 18 |
| `cargo check` | 0 | 3 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 199 |
| `cargo test` | 101 | 674 |

## Combined Kujo checks (final)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo fmt --check` | 0 | 16 |
| `cargo check` | 0 | 68 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 143 |
| `cargo test` | 0 | 980 |
| `cargo test --test docs_examples` | 0 | 13 |
| `cargo test --test readme_contracts` | 0 | 3 |
| `cargo test --test cli_contracts` | 0 | 10 |
| `cargo test --test cli_json_contracts` | 0 | 4 |
| `cargo test --test diagnostics_golden` | 0 | 2 |
| `cargo test --test vm_interpreter_parity_surfaces` | 0 | 4 |
| `cargo run -- test --runtime vm` | 0 | 286 |
| `cargo run -- test --runtime dual` | 0 | 37 |
| `cargo test --test closure_capture_audit --test closure_capture_contracts --test imported_vm_callback --test workflow_control_contracts --test runtime_semantics_characterization` | 0 | 10 |
| `bash scripts/release_gate.sh --full` | 0 | 727 |

## Fresh integrated artifact schema checks

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `cargo test --test workflow_control_contracts -- --include-ignored` | 0 | 5 |

## Integrated-runtime companion checks (Dispatch working directory)

| Exact command | Exit | Wall seconds |
|---|---:|---:|
| `"$KUJO_BIN" test-run tests/control_boundary_tests.kujo -v` | 0 | 44 |
| `"$KUJO_BIN" test-run tests/failure_gate_safety_tests.kujo -v` | 0 | 7 |
| `"$KUJO_BIN" test-run tests/failure_gate_execution_tests.kujo -v` | 0 | 30 |
| `"$KUJO_BIN" run tests/reexecution_lifecycle_fixture.kujo` | 0 | 48 |
| `bash tests/decision_claim_contract.sh` | 0 | 8 |
| `"$KUJO_BIN" run examples/failure-gate/run.kujo` | 0 | 41 |
| `"$KUJO_BIN" run tests/control_journal_benchmark.kujo` | 0 | 7 |
| `"$KUJO_BIN" run dispatch.kujo --help 2>/tmp/kujo-wave1/integrated-dispatch-help.stderr && test ! -s /tmp/kujo-wave1/integrated-dispatch-help.stderr` | 0 | 0 |
| `"$KUJO_BIN" run dispatch.kujo version 2>/tmp/kujo-wave1/integrated-dispatch-version.stderr && test ! -s /tmp/kujo-wave1/integrated-dispatch-version.stderr` | 0 | 1 |

## Workcell isolation

Runtime: Agent-3 Kujo source binary, `KUJO` explicitly set;
`WORKCELL_TEST_KUJO_VERSION=1.5.0` is a source compatibility override.
It does not certify Workcell's pinned 1.2.1 release.

| Exact command | Exit |
|---|---:|
| `./bin/workcell --help` | 0 |
| `./bin/workcell --version` | 0 |
| `./bin/workcell validate --file workcell.json` | 0 |
| `./tests/version_consistency.sh` | 0 |
| `./tests/run.sh` | 0 |
| `./tests/quality.sh` | 0 |
| `./tests/release_report.sh` | 0 |
| `./tests/markdown_links.sh` | 0 |
| `git diff --check` | 0 |
| `npm test --prefix adapters/official` | 0 |
| `npm run integrity:check --prefix adapters/official` | 0 |

Raw local logs and fixture pointers were retained under `/tmp/kujo-wave1`;
this committed ledger and the integration review preserve the durable outcomes.

## Failure classification and retained limitations

- Original Agent-1 full test/release runs: native tiny_http wildcard shutdown,
  independently reproduced without the VM; corrected by `ba7f962`.
- First corrected full test: stale unsafe inventory line after adding shutdown
  coverage; regenerated in `761c75a`. No new unsafe boundary.
- Subsequent corrected full test: LSP hover 132.35 ms versus its 120 ms guardrail;
  unchanged isolated rerun and full no-fail-fast rerun passed. No threshold change.
- First combined full test: three docgen redirect checks failed. An unchanged
  full docgen rerun passed 53/55; a filtered rerun passed 4/6. The pre-integration
  Agent-3 test binary passed all six twice. Diagnostic runs reproduced a fixture
  idle exit before any request; retaining the fixture alone did not fix failures.
  A diagnostic build then passed both positive redirect checks four times.
  Both temporary fixture and production tracing edits were discarded: evidence
  did not establish a source regression or justify a permanent workaround.
  The final uninstrumented suite/gate results above are authoritative. Earlier
  failures remain recorded rather than relabeled as passes.

Default ignores: seven JIT benchmark tests per library/binary test harness, one
Environment doctest, the known interpreter returned-named-recursion probe, and
two external workflow artifact checks. Both workflow artifact checks were run
separately with real fixture paths (13/13 passing, no ignores).

The VM/dual command suites each intentionally skip six test-run framework files:
`generators_test.kujo`, `iterators_test.kujo`, `jit_direct_recursion.kujo`,
`jit_loop_tests.kujo`, `jit_register_locals.kujo`, `stdlib_crypto_test.kujo`.
Those skips are not passes; the historical generator fixture parser failure
remains outside this wave. Live provider/OCI tests and Workcell's pinned-runtime
release matrix were not certified.

## Companion execution identity

Dispatch's isolation gate ran from `dispatch-failure-gate-evidence-control`:
`KUJO_BIN=<Agent-3 worktree>/target/debug/kujo DISPATCH_OFFLINE_FIXTURE=true bash scripts/run_release_gate.sh`
(exit 0). It included 101/101 sharded tests, focused control/safety/execution
suites and the workload/lifecycle fixtures. Workcell's canonical source-runtime
checks and 23 official-adapter tests passed; its release report recorded 249 tests.

Fresh integrated companion checks used the integration repository's
`target/debug/kujo` as `KUJO_BIN`, `DISPATCH_OFFLINE_FIXTURE=true`, sibling
Workcell/Eval/RunLedger/CaseFile roots through `FAILURE_GATE_*`, and Workcell's
`tests/fixtures/backend-protocol` at the front of PATH. The golden path ran the
action once, evaluated twice, preserved evidence and blocked downstream work.

Actual schema check environment: `KUJO_FAILURE_GATE_HANDOFF` was the generated
`failure-golden-1790398257806/handoff.json` in the macOS temporary directory;
`KUJO_FAILURE_GATE_DISPATCH_ROOT` was the reviewed Dispatch companion worktree.
Run: `run-1790398258086-6421`.
Handoff SHA-256: `177331d9a756a8df92a6db493dc7983e67cfdec30cfaea5d1739fbe455e1b0a5`.
The fixture is ephemeral; the source test and receipt identity are durable.

## Final release-wrapper result

`KUJO_ENABLE_SOCKET_TESTS=1 bash scripts/release_gate.sh --full`: PASS, exit 0,
727 seconds. Its second complete Rust test pass also included all 55 docgen tests.
`cargo audit --deny warnings --ignore RUSTSEC-2025-0141` passed; the exception is
the pre-existing unmaintained build-only bincode policy in the repository script.
Optional `cargo deny check` was skipped (tool absent), and the optional general
benchmark smoke was disabled. Separate matched-profile closure and journal
measurements appear in the review; neither skipped optional command is a pass.
Two pre-existing vendored tiny_http dependency warnings remain; strict root
all-target/all-feature clippy passed unchanged.
