# Matrix multiplication state audit — partial

All four matrix variants pass **1,152 complete matrix/checksum observations**
in VM, interpreter and JIT. An independent integer oracle checks **116,736
output cells exactly**, without tolerance. No runtime or port implementation
changed. This verifies crafted output/state controls, not performance parity.

The 24 matrix pairs combine dimensions 1, 2, 3, 7, 16 and 17 with zero,
left-identity, right-identity and signed patterned cases. Entries are exact binary
fractions; products and sums remain exactly representable in these controls.
Each pair runs Single/T4/T8/T16, two checksum seeds and two iterations in each
runtime. Dimensions smaller than worker count and uneven partitions exercise
empty jobs and result assembly. Inputs and retained snapshots remain unchanged.

The original wrapper executes beside a diagnostic clone exposing its complete
result after checksum calculation. Their accumulated results must agree. The
collector checks every matrix and checksum. An independent verifier accumulates
integer outer products scaled by 32, constructs the seven-decimal checksum input
with integer arithmetic and verifies every cell, case ID and raw shard receipt.
It checks all 72 process outcomes and their aggregate output identities.

## Timeout retained and diagnostic isolation

The original combined fixture completed 384 VM and 384 interpreter observations,
but JIT reached its 120,000ms scheduler deadline after 251 output lines. That
invocation remains incomplete; it is not counted as a pass or attributed to a
semantic defect. Its source, inputs, outputs and incomplete receipt are archived.

The accepted diagnostic runs each matrix pair in a separate process, preserving
all 24 pairs, four variants, seeds, iterations, assertions and the same per-process
deadline. This bounds fixture duration; it does not change official benchmark
inputs or deadlines. Repeated cross-case cache lifetime is not established by
these isolated processes. No speed inference follows from either run.

## Source differences and remaining gates

Pinned native Rust uses a Rayon pool created inside `matmul_parallel` and
parallelizes output rows; its wasm path falls back to sequential multiplication.
Goose starts persistent workers in `new_base`, distributes transposed columns,
and assembles column results. Kujo creates async row jobs and assembles rows.
These source differences require separate setup/transfer/allocation accounting;
equal matrix values do not prove equal work or worker lifetime. Neither comparator
was executed in this audit. Existing async compilation-economics evidence remains
separate from this complete-output proof.

Canonical matrix generation, general floating-point/rounding policy, zero/invalid
dimensions, executed Rust/Goose controls, transfer/work counts, production sizes
and timing remain open. All four structural-equivalence rows remain partial.

## Evidence

- Comparator source revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Accepted loop-fix CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  This is not the memory candidate. Runs overlap the resource-probe build and
  provide no timing or actual native-participation claim.
- `probes/audit-matmul-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces
  the sharded collector. The archive includes the independent verifier.
- `matmul-state-audit-2026-11-03.json.gz` retains 221 hash-verified artifacts,
  including the original timeout evidence and final complete outputs.
