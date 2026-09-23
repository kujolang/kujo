# Database lifecycle correction — 2026-09-22

## Scope and provenance

Repository: `kujolang/kujo`, branch `main`. Starting checkout:
`cf785c0a7953717af16b657cda05b85d628144c5` (1.4.0 development).
Remote integration baseline: `9ec04ff19b4bf87fbcbf04d7ad1d1290d177150c`
(1.5.0). The remote advanced during verification; the initial push was rejected,
and the changes were rebased while preserving the upstream release, filesystem
and lexical-scope fixes. Only changelog placement needed manual resolution.
Ending implementation SHA: `58c087b5d7af2a05d5d9fd2ad26a5a533044c5f6`.
Verification continued on 2026-09-23.
This report is committed separately from the implementation it audits.

The user explicitly expanded the earlier StoryDesk-only scope to fix Kujo's
underlying resource ownership and then remove StoryDesk's workaround. No release
tag, package publication or unrelated sibling modification was performed.

## Baseline

- `cargo test --locked`: **2,684 passed, zero failed, 15 ignored**, across 79
  test binaries/doc-test groups, before source edits. Existing ignored tests
  were retained.
- `cargo fmt --check`: passed before edits.
- Original release runtime fixture checks: **154/154** in VM and dual modes;
  six inventory skips, zero interpreter fallback in dual mode.
- Original live PostgreSQL 14 TLS/RLS/pooling/timeout gate: passed in both engines.
- Disposable local MariaDB 10.11.18: the original runtime failed its first query
  after connect in **both engines**, reporting a shut-down Tokio context.
- An additional isolated async PostgreSQL TLS reproduction on the original
  runtime aborted in interpreter mode: session initialization ran the synchronous
  client inside an active Tokio context; unwinding also dropped the client there.
- Original SQLite reproduction: `db_close` returned true while `lsof` still found
  the descriptor; only dropping the last reference released it.
- All eight existing hosted workflows passed on remote baseline `9ec04ff`,
  including [the release gate](https://github.com/kujolang/kujo/actions/runs/35794635670).
  The incoming changes did not modify database ownership or provider initialization.

## Findings and changes

| ID | Priority | Finding and evidence | Root correction | Status |
|---|---|---|---|---|
| DB-01 | P1 | `db_close` only returned true; retained aliases kept SQLite open and caused Windows cleanup failures in StoryDesk. | Shared optional native handle; explicit close serialized with queries; every alias becomes closed. | Implemented |
| DB-02 | P1 | Returning the same shared pool handle would let a stale alias close a later borrower's session once close became real. | Move the native session to a new lease handle on return, invalidating the old lease; preserve double/cross-pool rejection and accounting. | Implemented |
| DB-03 | P1 | MySQL's per-operation Tokio runtime was dropped immediately after connect; subsequent queries failed against the dead reactor. | Use the existing persistent async executor for connect, operations, rollback cleanup and disconnect. | Implemented |
| DB-04 | P1 | Async PostgreSQL TLS initialization reproduced a nested-runtime panic and process abort on the original runtime. | Keep connect, session initialization and error-path drop in the runtime-safe closure; make rollback cleanup use the same boundary. | Implemented |
| DB-06 | P1 | Live MySQL 8.4 rejected prepared `START TRANSACTION` with error 1295, after the reactor fix made the server reachable. | Use the text protocol for parameter-free transaction control and cleanup, matching mysql_async’s own transaction implementation; retain prepared application queries. | Implemented |
| DB-05 | P2 | The type checker warned that the existing `db_last_insert_id` builtin was undefined. | Register its existing one-argument integer-returning signature and add a behavioral type-check test. | Implemented |

Main implementation: `src/interpreter/database_handle.rs`, native `database.rs`,
`value.rs`, interpreter cleanup and `type_checker.rs`. The native mutex guards
prove an open resource while borrowed. Close retains the lock until native release
finishes; repeated close cannot acknowledge another still-running close. SQLite
close failure restores the live connection for retry. Network disconnect errors
are returned, and the consumed handle remains closed. Transaction bookkeeping,
close and pool return use a consistent transaction-state-then-resource lock order.

Tests cover retained variable/container aliases, repeated close, every database
operation after close, rollback without implicit commit, read-only handles,
immediate file removal, simultaneous close/query, stale pool leases, replacement
of explicitly closed leases, pool shutdown and existing lease accounting. Live
provider probes cover SQL parameters, transactions, session reuse, async calls,
TLS, RLS, reset, health eviction and timeouts. No sleeps or relaxed assertions were
added to conceal failures. Existing test failures were not marked expected.

## Compatibility and security

- Kujo builtin names, arguments, success shapes for open connections, SQL parameter
  handling, capabilities, CLI exit codes and machine-readable error schemas are
  unchanged. No persistence, config, environment or serialization format changed.
- Use after explicit close or pool return now fails. This intentionally corrects
  invalid resource use; callers must finish operations before close/return.
- Repeated close still succeeds. Closing a checked-out lease does not implicitly
  return its reservation: release it once through its owning pool.
- Rust embedders constructing `DatabaseConnection` directly must replace native
  `Arc<Mutex<T>>` constructors with `Arc<DatabaseHandle<T>>`. This representation
  change is explicit, not claimed source-compatible. Ecosystem searches found
  runtime copies but no independent Rust consumers constructing these handles.
- Capability gates and TLS authentication/hostname checks were preserved. No new
  unsafe code, shell execution, credential input or dependency was introduced.
- The upstream 1.5.0 changes are preserved and verified with the integrated tree;
  they are not attributed to this database fix.

## Measured impact

The same `lsof` reproduction changes from **descriptor present after close** to
**descriptor absent after close**, while aliases remain reachable. StoryDesk's new
regression fails on the original runtime (**23 passed / 1 failed**) and passes with native close (**24 passed / zero failed**). MySQL's first query changes from a reproducible failure
to success in both engines. Per-call MySQL reactor construction is removed by
inspection; no unsupported latency, CPU, memory-percentage or token savings are
claimed. Runtime dependencies and lockfile changes attributable to this fix: zero.
Pool return allocates a fresh shared handle to isolate leases; it retains the
physical connection, verified by session identity in the live MySQL probe;
PostgreSQL reset/health tests independently verify reusable pooled behavior. This is a deliberate ownership cost, not
an unmeasured claim of faster pooling.

## Verification and development corrections

An intermediate full-suite run caught one documentation-inventory mismatch: the
new `db_close` row said `exact 1`, while centralized metadata classifies its arity
as `handler-defined`. The documentation was corrected to match the existing
metadata, without changing the handler or weakening the inventory test. An early
async probe also used `await` inside a comparison incorrectly; awaiting into a
binding exposed the actual baseline PostgreSQL abort described above.

The next full-suite run caught stale generated TODO inventory line numbers after
registering the builtin. Both generated files were refreshed with the existing
strict generator; marker classifications and the freshness assertion were retained.
The first hosted provider run passed MariaDB but exposed MySQL 8.4 error 1295 and
a PostgreSQL cluster-start failure. Transaction control was corrected as DB-06.
The test cluster now owns its socket directory instead of relying on the platform
default and emits its server log on startup failure. These are recorded failures,
not successful verification receipts.

The Linux negative-hostname gate additionally exposed inconsistent fixture
handling of thrown connection errors versus returned error values. The probe now
labels errors from the connection operation explicitly and retains the native
cause. Hostname rejection and credential-redaction assertions are preserved.
Test-server restarts consistently use the private socket directory, and failure
receipts identify the failing shell line rather than discarding captured evidence.
Runtime code is unchanged by these test-harness follow-ups.

## Final local verification

Working directory: `kujo`. Implementation source is `58c087b`; subsequent changes
in `c99959d` and `c101b7e` affect only the PostgreSQL integration harness.

| Command | Result |
|---|---|
| `cargo test --locked` | 2,717 passed, zero failed, 15 ignored; 82 test/doc-test groups |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed |
| `cargo check --locked --no-default-features` | Passed |
| `cargo fmt --check` | Passed |
| `target/debug/kujo test --runtime vm` | 154/154; six inventory skips |
| `target/debug/kujo test --runtime dual` | 154/154; zero interpreter fallback |
| `KUJO="$PWD/target/debug/kujo" bash tests/postgres_tls.sh` | Passed TLS, RLS, pooling, lifecycle and negative-certificate checks |
| `bash .audit-evidence/db-close/mysql-verify.sh` | Disposable local MariaDB: both engines passed, including async calls |
| `cargo build --locked --release` | Passed; 10m 26s on this host (no comparative speed claim) |
| `git diff --check` | Passed |

The local MySQL helper creates and tears down its own loopback MariaDB data
directory and runs `tests/mysql_lifecycle_probe.kujo` in both engines with
`KUJO_MYSQL_TEST_URL` pointing only to that disposable database. CI independently
executes the committed probe against clean MySQL 8.4 and MariaDB 10.11 containers.
No live credentials or external databases were used.

Test-count growth also includes upstream 1.5.0 changes; it is not all attributed
to this patch. This patch adds four Rust lifecycle integration tests, one type
checker regression (compiled in both library and binary suites), and live-provider
assertions. Existing vendor/minimal-build warnings were not suppressed.

## Hosted verification

Receipts identify the tested source, including failed intermediate attempts rather
than treating a partially failed workflow as green:

| Source / gate | Evidence | Result |
|---|---|---|
| Runtime `58c087b`, full release gate | [35903290800](https://github.com/kujolang/kujo/actions/runs/35903290800) | Release, minimal smoke, clippy, format, VM/interpreter parity all passed |
| Runtime `58c087b`, MySQL 8.4 and MariaDB 10.11 | [35903290788](https://github.com/kujolang/kujo/actions/runs/35903290788) | Both provider jobs passed; PostgreSQL harness failure in this run was corrected separately |
| Runtime unchanged, final PostgreSQL harness `c101b7e` | [job 107332334100](https://github.com/kujolang/kujo/actions/runs/35905588234/job/107332334100) | Passed live PostgreSQL TLS, pooling, RLS, lifecycle and negative trust checks in both engines |
| Runtime `58c087b`, native five-platform matrix | [35903290748](https://github.com/kujolang/kujo/actions/runs/35903290748) | Passed Linux x64/ARM, macOS x64/ARM and Windows; includes all four database lifecycle tests on each platform |
| Runtime `58c087b`, filesystem capability conformance | [35903290724](https://github.com/kujolang/kujo/actions/runs/35903290724) | Passed |
| Runtime `58c087b`, release artifact matrix | [35903290856](https://github.com/kujolang/kujo/actions/runs/35903290856) | Passed |
| Runtime `58c087b`, LSP contract matrix | [35903290620](https://github.com/kujolang/kujo/actions/runs/35903290620) | Passed |
| StoryDesk `d3bd1e9`, pinned runtime `58c087b` | [35903383436](https://github.com/kujolang/storydesk/actions/runs/35903383436) | Linux/macOS/Windows passed validation and both-adapter 24-writer contention; Linux passed 12 paired qualification cases |

Artifact, release-state and field-notes guards also passed on `58c087b`. MySQL
and MariaDB jobs queued again by test-harness-only commits are redundant with the
successful runtime-source receipt above. No runtime code changed after `58c087b`.

Local verbose evidence is retained in ignored `.audit-evidence/db-close/`.
StoryDesk's detailed adoption receipt is in its repository at
`docs/audits/native-database-lifetime-2026-09-23.md`.

## Cross-repository closure and remaining work

StoryDesk adopted the exact source revision, removed reference-clearing
mitigations, and added four native-close regression checks. Its release-runtime
local validation and hosted three-platform matrix passed. No additional sibling
change is needed for this correction.

No known introduced regression or unresolved P0/P1/P2 implementation finding
remains in this database-lifecycle scope. Existing ignored tests remain visible.
No speculative P3 cleanup was included. Publishing a future Kujo release is a
separate release-management action; StoryDesk's documented source pin already
selects the fixed runtime. The published 1.5.0 tag itself predates these changes.
This is evidence of tested compatibility, not a claim that finite tests prove
absence of every possible regression.
