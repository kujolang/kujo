# Database connection lifetime

`db_close(db)` releases the native SQLite, PostgreSQL or MySQL connection before
returning `true`. Aliases in variables, containers and closures share that lifetime.
Repeated close returns `true`. Queries, execution, transaction operations and
last-insert-ID access through a closed alias fail with a database-closed error.
Closing a connection rolls back uncommitted work; it does not commit it.

Previously, `db_close` returned `true` without releasing anything until the final
reference disappeared. Programs that deliberately used a connection after closing
it depended on that bug and must keep it open until their last operation. Assigning
`null` after close is unnecessary, including when deleting SQLite files on Windows.
No syntax, capability, CLI exit-code, JSON, configuration or environment contract
changes accompany this correction. Open-connection SQL and parameter behavior is
unchanged. This is a native resource-lifetime bug correction under the runtime
compatibility policy, with the formerly successful use-after-close behavior
intentionally rejected.

## Pools

`db_pool_release(pool, db)` invalidates that lease and every alias. The underlying
connection can be reset and reused, but a fresh handle represents the next lease.
An old alias cannot query, modify or close another borrower's connection.
Double release and release to another pool remain errors.

`db_close` on a checked-out connection closes its native resource but does not
implicitly return its pool reservation. Call `db_pool_release(pool, db)` once to
release that reservation; the pool creates a replacement when needed. Pool close
continues to reject new acquisitions while allowing existing borrowers to finish
and release their leases. PostgreSQL session reset and TLS policy are unchanged.

Operations and close serialize on the native resource. Transaction state changes,
close and pool return also serialize on the shared transaction-state lock. A
query that wins the resource lock completes before close; one that loses receives
a closed error. SQLite close failure restores the live handle for retry. Network
close failures are reported; the consumed network handle remains closed.

MySQL connections use Kujo's persistent async executor for creation, operations,
cleanup and disconnect. The executor remains alive across native calls and handles
calls from synchronous code and Tokio contexts.

## Embedding and verification

The Rust `DatabaseConnection` enum now contains `Arc<DatabaseHandle<T>>` rather than
`Arc<Mutex<T>>`. Rust embedders constructing native values directly must use
`DatabaseHandle::new`; ordinary Kujo code and SDK consumers require no signature
change. No external Rust constructors were found in the inspected ecosystem
checkout (other runtime checkouts contain their own copies).

Run `cargo test --locked --test database_lifecycle` for retained references,
rollback, pool lease isolation, concurrency and both language engines. This test
runs in the existing five-platform native CI matrix. The provider CI additionally
runs `tests/mysql_lifecycle_probe.kujo` against MySQL 8.4 and MariaDB 10.11 in both
engines, and `bash tests/postgres_tls.sh` against PostgreSQL 14 with verified TLS,
RLS, pooling and timeout checks. Provider containers follow their maintained
minor release tags; the Rust dependency graph remains locked.
