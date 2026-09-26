#!/usr/bin/env bash
# Disposable loopback MariaDB validation; never uses an existing data directory.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KUJO="${KUJO:-${ROOT}/target/debug/kujo}"
TEST_ROOT="$(mktemp -d "/tmp/kujo-mysql-lifecycle.XXXXXX")"
SERVER_PID=''
cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT
# Maintenance-only Python selects an unused loopback port; runtime probes remain Kujo.
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
mariadb-install-db --no-defaults --datadir="$TEST_ROOT/data" \
    --auth-root-authentication-method=normal --skip-test-db >"$TEST_ROOT/install.log" 2>&1
mysqld --no-defaults --datadir="$TEST_ROOT/data" --socket="$TEST_ROOT/mysql.sock" \
    --pid-file="$TEST_ROOT/mysql.pid" --port="$PORT" --bind-address=127.0.0.1 \
    --log-error="$TEST_ROOT/server.log" >"$TEST_ROOT/stdout.log" 2>&1 &
SERVER_PID=$!
READY=0
for ((attempt=0; attempt<30; attempt++)); do
    if mysqladmin --no-defaults --socket="$TEST_ROOT/mysql.sock" --user=root ping >/dev/null 2>&1; then
        READY=1
        break
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then break; fi
    sleep 1
done
if [[ "$READY" != 1 ]]; then
    cat "$TEST_ROOT/server.log" "$TEST_ROOT/stdout.log" >&2
    exit 1
fi
mysql --no-defaults --socket="$TEST_ROOT/mysql.sock" --user=root -e 'CREATE DATABASE kujo_lifecycle'
export KUJO_MYSQL_TEST_URL="mysql://root@127.0.0.1:${PORT}/kujo_lifecycle"
"$KUJO" run "$ROOT/tests/mysql_lifecycle_probe.kujo"
"$KUJO" run "$ROOT/tests/mysql_lifecycle_probe.kujo" --interpreter
