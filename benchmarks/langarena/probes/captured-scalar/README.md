# Captured-scalar publication contracts

These fixtures guard future imported-helper batching changes. Run each `main.kujo`
with `kujo run <path> --untrusted`, then repeat with `--interpreter` and `--jit`.

- `type-change`: the same helper sees a captured number, a struct with `op_add`,
  then a number again. The callback must observe preceding dictionary writes;
  its own writes must preserve retained snapshots. Expected stdout:
  `CAPTURE TYPE CHANGE PASS`.
- `immutable-write`: assignment to a captured constant must fail while earlier
  dictionary writes remain visible and previous snapshots remain unchanged.
  Expected stdout: `CAPTURE IMMUTABLE WRITE PASS`.

These are semantic contracts, not benchmark performance results. They originate
from `.audit-evidence/captured-scalar-2026-10-29/captured_scalar_publication.rs`.
They specifically guard per-invocation type checks and error publication; passing
these cases alone does not prove all captured-scalar optimizations safe.

All 12 combinations passed on accepted runtime `e445f4a` and the unaccepted
array-batch V2 candidate. Binary and fixture hashes, commands, exit statuses and
outputs are retained in
`.audit-evidence/captured-scalar-portable-2026-10-29/report.json`.
