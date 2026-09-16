#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KUJO="${KUJO:-${ROOT}/target/debug/kujo}"
TMP_ROOT="$(mktemp -d)"
DATA_DIR="${TMP_ROOT}/data"
PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')"
SERVER_RUNNING=0

cleanup() {
    if [[ "${SERVER_RUNNING}" == "1" ]]; then pg_ctl -D "${DATA_DIR}" -m immediate stop >/dev/null 2>&1 || true; fi
    rm -rf "${TMP_ROOT}"
}
trap cleanup EXIT

initdb -D "${DATA_DIR}" --no-locale --encoding=UTF8 -A trust >/dev/null
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj '/CN=Kujo PostgreSQL Test CA' -keyout "${TMP_ROOT}/ca.key" -out "${TMP_ROOT}/ca.pem" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -subj '/CN=localhost' -keyout "${TMP_ROOT}/server.key" -out "${TMP_ROOT}/server.csr" >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n' >"${TMP_ROOT}/server.ext"
openssl x509 -req -days 1 -in "${TMP_ROOT}/server.csr" -CA "${TMP_ROOT}/ca.pem" -CAkey "${TMP_ROOT}/ca.key" -CAcreateserial -extfile "${TMP_ROOT}/server.ext" -out "${TMP_ROOT}/server.pem" >/dev/null 2>&1
chmod 600 "${TMP_ROOT}/server.key" "${TMP_ROOT}/ca.pem"

pg_ctl -D "${DATA_DIR}" -o "-h 127.0.0.1 -p ${PORT} -c ssl=on -c ssl_cert_file='${TMP_ROOT}/server.pem' -c ssl_key_file='${TMP_ROOT}/server.key'" -w start >/dev/null
SERVER_RUNNING=1

psql -h 127.0.0.1 -p "${PORT}" -d postgres -v ON_ERROR_STOP=1 <<'SQL' >/dev/null
CREATE ROLE qf_app LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT;
CREATE TABLE qf_tenant_rows (organization_id text NOT NULL, value text NOT NULL);
CREATE TABLE qf_commands (organization_id text NOT NULL, command_id text PRIMARY KEY);
CREATE TABLE qf_events (organization_id text NOT NULL, event_id text PRIMARY KEY);
INSERT INTO qf_tenant_rows VALUES ('tenant_a', 'A'), ('tenant_b', 'B');
ALTER TABLE qf_tenant_rows ENABLE ROW LEVEL SECURITY;
ALTER TABLE qf_tenant_rows FORCE ROW LEVEL SECURITY;
ALTER TABLE qf_commands ENABLE ROW LEVEL SECURITY;
ALTER TABLE qf_commands FORCE ROW LEVEL SECURITY;
ALTER TABLE qf_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE qf_events FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_rows_policy ON qf_tenant_rows USING (organization_id = current_setting('app.organization_id', true)) WITH CHECK (organization_id = current_setting('app.organization_id', true));
CREATE POLICY commands_policy ON qf_commands USING (organization_id = current_setting('app.organization_id', true)) WITH CHECK (organization_id = current_setting('app.organization_id', true));
CREATE POLICY events_policy ON qf_events USING (organization_id = current_setting('app.organization_id', true)) WITH CHECK (organization_id = current_setting('app.organization_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON qf_tenant_rows, qf_commands, qf_events TO qf_app;
SQL

export KUJO_POSTGRES_TLS_CA_FILE="${TMP_ROOT}/ca.pem"
export KUJO_POSTGRES_TLS_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=$(id -un) dbname=postgres connect_timeout=3 sslmode=disable"
for engine in vm interpreter; do
    if [[ "${engine}" == "interpreter" ]]; then
        output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"
    else
        output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --allow-db --allow-fs --allow-env 2>&1)"
    fi
    python3 -c 'import json,sys; value=json.loads(sys.argv[1].splitlines()[-1]); assert value == {"ok":True,"schema":"dev.kujolang.postgres-tls-probe.v1","tls":"verified","value":1}' "${output}"
done

export KUJO_POSTGRES_TLS_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=qf_app dbname=postgres connect_timeout=3 sslmode=disable"
for engine in vm interpreter; do
    export KUJO_POSTGRES_TEST_RUN="${engine}"
    if [[ "${engine}" == "interpreter" ]]; then
        output="$(${KUJO} run "${ROOT}/tests/postgres_tls_pool_rls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"
    else
        output="$(${KUJO} run "${ROOT}/tests/postgres_tls_pool_rls_probe.kujo" --allow-db --allow-fs --allow-env 2>&1)"
    fi
    python3 -c 'import json,sys; value=json.loads(sys.argv[1].splitlines()[-1]); assert value == {"atomicity":"verified","ok":True,"rls":"enforced","schema":"dev.kujolang.postgres-tls-pool-rls.v1","session_reset":"verified","tls":"verified"}' "${output}"
done

export KUJO_POSTGRES_TLS_URL="host=127.0.0.1 port=${PORT} user=$(id -un) dbname=postgres connect_timeout=3 password=hostname-secret"
if output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"; then
    echo "hostname mismatch unexpectedly succeeded" >&2
    exit 1
fi
[[ "${output}" != *"hostname-secret"* ]]
[[ "${output}" == *"verified PostgreSQL TLS connection failed"* ]]

openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj '/CN=Wrong Kujo CA' -keyout "${TMP_ROOT}/wrong-ca.key" -out "${TMP_ROOT}/wrong-ca.pem" >/dev/null 2>&1
chmod 600 "${TMP_ROOT}/wrong-ca.pem"
export KUJO_POSTGRES_TLS_CA_FILE="${TMP_ROOT}/wrong-ca.pem"
export KUJO_POSTGRES_TLS_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=$(id -un) dbname=postgres connect_timeout=3 password=ca-secret"
if output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"; then
    echo "untrusted CA unexpectedly succeeded" >&2
    exit 1
fi
[[ "${output}" != *"ca-secret"* ]]

pg_ctl -D "${DATA_DIR}" -m fast -w stop >/dev/null
SERVER_RUNNING=0
mkdir -p "${TMP_ROOT}/newcerts"
: >"${TMP_ROOT}/ca-index.txt"
printf '1000\n' >"${TMP_ROOT}/ca-serial"
cat >"${TMP_ROOT}/ca.cnf" <<EOF
[ ca ]
default_ca = kujo_ca
[ kujo_ca ]
database = ${TMP_ROOT}/ca-index.txt
serial = ${TMP_ROOT}/ca-serial
new_certs_dir = ${TMP_ROOT}/newcerts
certificate = ${TMP_ROOT}/ca.pem
private_key = ${TMP_ROOT}/ca.key
default_md = sha256
policy = kujo_policy
x509_extensions = server_ext
[ kujo_policy ]
commonName = supplied
[ server_ext ]
subjectAltName = DNS:localhost
extendedKeyUsage = serverAuth
EOF
openssl ca -batch -config "${TMP_ROOT}/ca.cnf" -startdate 20200101000000Z -enddate 20200102000000Z -in "${TMP_ROOT}/server.csr" -out "${TMP_ROOT}/expired-server.pem" >/dev/null 2>&1
pg_ctl -D "${DATA_DIR}" -o "-h 127.0.0.1 -p ${PORT} -c ssl=on -c ssl_cert_file='${TMP_ROOT}/expired-server.pem' -c ssl_key_file='${TMP_ROOT}/server.key'" -w start >/dev/null
SERVER_RUNNING=1
export KUJO_POSTGRES_TLS_CA_FILE="${TMP_ROOT}/ca.pem"
export KUJO_POSTGRES_TLS_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=$(id -un) dbname=postgres connect_timeout=3 password=expired-secret"
if output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"; then
    echo "expired PostgreSQL certificate unexpectedly succeeded" >&2
    exit 1
fi
[[ "${output}" != *"expired-secret"* ]]

pg_ctl -D "${DATA_DIR}" -m fast -w stop >/dev/null
SERVER_RUNNING=0
export KUJO_POSTGRES_TLS_CA_FILE="${TMP_ROOT}/ca.pem"
pg_ctl -D "${DATA_DIR}" -o "-h 127.0.0.1 -p ${PORT} -c ssl=off" -w start >/dev/null
SERVER_RUNNING=1
export KUJO_POSTGRES_TLS_URL="host=localhost hostaddr=127.0.0.1 port=${PORT} user=$(id -un) dbname=postgres connect_timeout=3 password=plaintext-secret"
if output="$(${KUJO} run "${ROOT}/tests/postgres_tls_probe.kujo" --interpreter --allow-db --allow-fs --allow-env 2>&1)"; then
    echo "plaintext PostgreSQL unexpectedly succeeded" >&2
    exit 1
fi
[[ "${output}" != *"plaintext-secret"* ]]

echo "verified PostgreSQL TLS: passed"
