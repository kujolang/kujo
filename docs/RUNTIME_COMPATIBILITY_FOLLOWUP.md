# Phase-B compatibility follow-up

Starting commit: `3efa60cf2e7dc89607a9988509b16cc125375140`.
Branch: `runtime/generator-async-spawn-completion`. Main is unchanged.

This follow-up implements the user's request to fix the items retained by the
previous completion report. It does not change closure snapshot identity or
introduce atomic concurrent mutation, async generators or a sandbox guarantee.

## Corrections

- Static checking uses runtime builtin availability and aliases, lexical callable
  bindings, nested declarations, anonymous-function parameters, match and except
  bindings, explicit annotation constraints and the actual two-argument
  `io_seek_read` contract. Unknown gradual result types no longer cause false
  `?` diagnostics. Explicit annotation violations, missing functions and known
  invalid callable/try operands remain errors.
- Both AST call forms reach the existing ArgParser implementation. Arguments are
  evaluated once and failures propagate. VM relational operations invoke existing
  struct operator methods before primitive comparison.
- The operator fixture now reaches its successful ending instead of accepting
  a VM error halfway through. Its Point formatter explicitly converts numbers,
  retaining the language's requirement for string concatenation operands.
- Interpreter failures preserve frames before unwinding; handling an error clears
  that trace. Five diagnostic fixtures have exact interpreter expectations for
  subsystem codes or deliberately invalid expressions. No warning/error text is
  stripped to make a fixture pass. The full release gate now runs the explicit
  interpreter fixture sweep in addition to its VM-primary dual sweep.
- Docgen's timeout was reproduced in an experimental single-worker raw HTTP
  fixture, then in standalone Tokio, Mio and standard-library TCP probes.
  The experimental fixture was reverted; production Docgen and its existing
  deadlines/assertions are unchanged. A dependency-free diagnostic is retained
  in `scripts/loopback_network_probe.rs`.
- A committed disposable MariaDB harness ignores ambient configuration and tests
  both engines on a fresh loopback database. PostgreSQL uses its existing pinned
  major-14 disposable TLS/RLS/pooling harness.

## Validation

Initial targeted results: checker unit suite 30/30; CLI contracts 32/32;
compatibility regressions 5/5; parity surfaces 117/117; Docgen 55/55.
The explicit interpreter sweep passes 149/149, eleven dedicated skips.
PostgreSQL TLS and MariaDB lifecycle harnesses pass in both engines.

Docgen stress is **FAIL**, not a retried pass: the raw-fixture experiment passed
12 captured full-suite iterations and failed iteration 13 (53/55). Standalone
Tokio failed on iteration 11; raw Mio and standard TCP also timed out. Ad-hoc
signing a disposable standard TCP probe did not fix it (failure on iteration 42).
Native sampling showed request threads waiting and async reactors parked in
`kevent`; servers accepted connections but received no bytes. Standard TCP
`connect_timeout` also expired, establishing that Kujo/Docgen is not required to
reproduce the failure. The two-second deadlines were never increased.

LuLu and Tailscale network extensions are active. LuLu logged unsigned-code
validation errors for affected test executables, but that does not establish
which component causes the stall; signing did not eliminate it. No firewall or
network-filter policy was modified. A clean host or review of this host's
network filtering is needed before treating the networking stress gate as green.

The committed dependency-free probe passed 50 iterations from `/tmp`, but failed
on iteration 8 when built in `target/debug/deps`, where Cargo test executables
live. Both concurrent connections timed out. This path-sensitive observation is
additional evidence for host-environment review, not proof of a particular rule.

The first full release-gate attempt stopped at a documentation contract: a required
unchanged runtime-decision marker had been rephrased. The exact marker was restored
without changing the decision or weakening the assertion. The final full gate
passed after that correction and restoration of the original Docgen fixture.

Final verification used `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
RUST_TEST_THREADS=2`; the release wrapper also used `KUJO_ENABLE_SOCKET_TESTS=1`.

| Command | Final result |
| --- | --- |
| `cargo fmt --check` | PASS, in the release wrapper |
| `cargo check` | PASS |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS, in the release wrapper |
| `cargo test` | PASS; library 914 passed / 7 ignored; binary 947 passed / 7 ignored; integration targets passed |
| `cargo test` integration targets: `docs_examples`, `readme_contracts`, `cli_contracts`, `cli_json_contracts`, `diagnostics_golden` | PASS within the full suite: 6, 1, 32, 19 and 2 tests respectively |
| `cargo test` integration targets: `docgen_universal`, `interpreter_compatibility_regressions` | PASS on the final tree: 55 and 5 tests respectively |
| `cargo test --test vm_interpreter_parity_surfaces` | PASS, 117 tests |
| `cargo run -- test --runtime vm` | PASS, 149/149; 11 explicit skips |
| `cargo run -- test --runtime dual` | PASS, 149/149; 11 explicit skips |
| `cargo run -- test --runtime interpreter` | PASS, 149/149; 11 explicit skips |
| `cargo test --test workflow_control_contracts -- --include-ignored` | PASS, 13/13 using fresh cross-component fixture artifacts |
| `bash tests/postgres_tls.sh` | PASS, both engines against disposable PostgreSQL 14.20 |
| `bash tests/mysql_lifecycle.sh` | PASS, both engines against disposable MariaDB 10.11.18 |
| `bash scripts/release_gate.sh --full` | PASS, exit 0; socket tests enabled (31/31); audit passed with the existing build-only bincode advisory exception |
| `rustc --edition=2021 -D warnings --test scripts/loopback_network_probe.rs -o /tmp/kujo-final-host-probe-build` | PASS, diagnostic compiles without warnings; this is not a networking pass |

The full wrapper skipped optional benchmark smoke (not enabled) and `cargo deny`
(not installed). Existing ignored tests and the eleven fixture skips are not
claimed as passes. The explicit workflow invocation includes both otherwise
ignored artifact checks. Its environment pointed `KUJO_FAILURE_GATE_HANDOFF` at
the fresh `failure-golden-1790423304738/handoff.json` and
`KUJO_FAILURE_GATE_DISPATCH_ROOT` at the sibling
`dispatch-failure-gate-evidence-control` checkout. The deterministic golden
fixture itself returned `ok: true` with run ID `run-1790423304954-7697`.

Evidence logs on this host are `/tmp/kujo-open-release-final.log`,
`/tmp/kujo-open-{check,vm,dual,interpreter-cargo,workflow,postgres,mysql}-final.log`
and `/tmp/kujo-open-host-probe-target-failure.log`. These are local diagnostics,
not permanent artifacts; this committed record preserves their relevant outcomes.

**Overall remaining-item status: BLOCKED.** The release wrapper passed, but the
separate networking stress failure remains unresolved and remote-provider
certification has not run. A passing final wrapper does not erase that failure.

## Review boundaries

The checker does not execute builtin code. Error traces retain bounded strings
only on failure. ArgParser dispatch retains ordinary runtime argument evaluation;
comparison overloads use the existing bounded callable bridge. No capability,
workflow replay, provider authentication or effect policy is relaxed.

No comparative performance claim is made. Static checking remains interpreter-only;
ordinary VM local access, closure captures and startup paths are unchanged. Error
frame copying occurs only on failure. Struct relational overloads incur their
normal function-call cost; primitive comparisons only add the struct-dispatch check.

Local database evidence is distinct from remote Workcell provider certification.
Remote certification remains blocked on explicit provider/profile/account, region,
image and budget configuration. Docker is installed but its daemon is unavailable;
this does not prevent the installed PostgreSQL/MariaDB harnesses.

Historical SignalBox timeout records are
`cap_529acae7-227c-4096-afe8-5068c1efee7d` and
`sig_b0b21419-26ba-4e41-99eb-3c0cb8520ac1`. No duplicate capture or automatic
SignalBox disposition is created; the follow-up evidence is saved in Strata.
