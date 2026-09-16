# Verified-TLS PostgreSQL pool

Use `db_pool_postgres_tls` for production PostgreSQL traffic. The legacy
`db_pool("postgres", ...)` entry point is retained for compatibility and does
not provide this security contract.

```kujo
let ca_pem := read_file("/run/secrets/postgres-ca.pem")
let pool := db_pool_postgres_tls(
  reveal(secret(env_required("DATABASE_URL"))),
  ca_pem,
  {
    "min_connections": 2,
    "max_connections": 20,
    "acquisition_timeout_ms": 2000,
    "connect_timeout_ms": 10000,
    "idle_timeout_seconds": 300,
    "max_lifetime_seconds": 1800,
    "statement_timeout_ms": 30000,
    "health_check": true
  }
)

let db := db_pool_acquire(pool)
// Use parameterized db_query/db_execute calls and a transaction-local tenant setting.
db_pool_release(pool, db)
```

The API requires an explicit TCP hostname and a non-empty PEM CA bundle of at
most 1 MiB. It forces PostgreSQL TLS, TLS 1.2 or newer, certificate-chain
verification, and hostname verification. It does not fall back to plaintext or
system trust. Connection strings are bounded to 8 KiB. Failures are deliberately
content-free so credentials and certificate material are not returned.

## Options

Unknown keys and invalid types fail closed.

| Option | Default | Bounds | Meaning |
| --- | ---: | ---: | --- |
| `min_connections` | 2 | 0–64 | Connections created and verified before construction succeeds. |
| `max_connections` | 20 | 1–128 | Atomic upper bound across available, connecting, and leased connections. |
| `acquisition_timeout_ms` | 30000 | 100–300000 | Maximum wait for a lease. |
| `connect_timeout_ms` | 10000 | 100–300000 | Maximum connection/TLS establishment time. |
| `idle_timeout_seconds` | 300 | 1–86400 | Evict an available connection after this idle age. |
| `max_lifetime_seconds` | 1800 | 1–86400 | Evict a connection after this total age. |
| `statement_timeout_ms` | 30000 | 1–300000 | PostgreSQL server-side statement deadline, restored after reset. |
| `health_check` | true | Boolean | Run `SELECT 1` before checkout. |

`connection_timeout` remains a deprecated seconds-based compatibility alias
for the acquisition timeout when `acquisition_timeout_ms` is absent. New code
should not use it.

## Lease and reset contract

Each acquired database value is a single lease and must be released exactly
once to the same pool. Double release, cross-pool release, acquire after close,
and reuse of a released lease are application errors. Before reuse, PostgreSQL
connections execute `ROLLBACK`, `DISCARD ALL`, and restore `statement_timeout`.
This removes transaction-local tenant context, session settings, prepared
statements, and other borrower state. A failed reset, health check, expired
lifetime, closed socket, or poisoned connection causes eviction rather than
reuse.

`db_pool_close` stops new acquisitions and closes all currently available
connections. Connections already leased are discarded when returned. Shutdown
code must first stop accepting work, wait for application requests/jobs to
drain within its own bounded shutdown deadline, and then close the pool.

`db_pool_stats` returns `available`, `in_use`, `total`, `min`, `max`, `closed`,
`acquire_timeouts`, `connection_errors`, `evictions`, `health_check_failures`,
and `reset_failures`. Values contain no connection string or certificate data.

## Tenant-isolation gate

Run `bash tests/postgres_tls.sh`. The self-contained gate uses a temporary real
PostgreSQL server, generated test-only CA/server certificates, separate owner
and application roles, forced RLS policies with `USING` and `WITH CHECK`, and
both Kujo runtimes. It verifies hostile TLS cases, complete CRUD and worker-claim
isolation, absent tenant context, pooled-state reset, atomic state/event/outbox
writes and rollback, statement timeout, connection recovery, and lifetime
eviction. It requires local PostgreSQL server tools and OpenSSL; no database
daemon or certificate survives teardown.

The gate pins PostgreSQL major 14 by default and rejects a different server
major so local and CI evidence is comparable. Install PostgreSQL 14 client and
server tools on `PATH`, or set `POSTGRES_TEST_MAJOR` explicitly when qualifying
a deliberate future-major upgrade. OpenSSL is required only to generate the
ephemeral test CA and hostile certificate fixtures.
