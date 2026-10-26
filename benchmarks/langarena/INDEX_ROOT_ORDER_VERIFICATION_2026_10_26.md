# Indexed assignment reads the replacement root

Runtime `3a9cbe4` implements the approved rule: evaluate the right-hand value and
index, then load the target binding. If the index replaces the array, dictionary
or byte buffer, assignment updates that replacement. Local in-place indexing
already followed this order. Compound assignment retains its existing separate
read/write evaluation, including two evaluations of a side-effecting index.

The compiler uses a non-cloning `Swap` to restore the existing setter operand
order. VM dispatch validates the current call-frame boundary on malformed swaps;
JIT translation exchanges stack values without adding ownership references.
Real snapshots and existing mutation, bounds and error checks remain intact.

## Verification

Baseline runtime `8b4111b` passes 10/10 interpreter cases but only 1/10 in each
VM/JIT-enabled mode. The candidate passes all 30 cases, covering replacements
that grow/shrink, dictionaries, bytes, RHS/index order, immutable bindings,
throwing indices, snapshots, compound assignment and scoped global writes.

A separate forced native-function check executes 157 native write entries in
both binaries. The baseline yields `[42, 2, 3]`; the candidate yields the required
`[42, 9, 9]`. The index helper deliberately contains unsupported `try/except`, so
its execution cannot supply the native-function count. This proof is separate
from the JIT-enabled timing runs.

The normal release build, `cargo check`, formatting and diff checks passed:

- Library: 1,114 passed, zero failed, eight ignored.
- Integration: 206 passed, zero failed, one ignored, including malformed Swap
  operands and suspended caller-stack isolation.
- Unchanged test-size LangArena suite: 50/50 in VM, interpreter and JIT (150 total).
- Allocation: all 24 baseline/candidate ownership pairs pass; measured phase-4
  request counts and requested bytes are identical.

## Local performance

Seven alternating before/after pairs per workload/runtime used frozen normal
release binaries. Ratio is candidate time divided by baseline; below 1 is faster.
No median exceeded the 1.05 slowdown review gate.

| Global update workload | VM median | Interpreter median | JIT-enabled median |
| --- | ---: | ---: | ---: |
| Array | 1.021725 | 0.979487 | 0.959291 |
| Dictionary | 1.005104 | 0.999887 | 0.924854 |

VM array writes cost about 2.2% more in these local observations. JIT array ratios
range from 0.725427 to 1.269888: the final pair was slower, while the other six
were faster. That is variability, not proof of a general speedup or a repeatable
slowdown above the gate. All samples remain in the evidence.

Eight separate peak-RSS observations (bytes):

| Workload/runtime | Baseline | Candidate |
| --- | ---: | ---: |
| Array VM | 6,762,496 | 6,877,184 |
| Array JIT | 10,895,360 | 10,960,896 |
| Dictionary VM | 7,823,360 | 7,864,320 |
| Dictionary JIT | 12,374,016 | 12,402,688 |

These single RSS observations do not measure retained heap. Allocation probes
use matching thin-LTO builds. An initial LTO-off diagnostic failed because the
Apple linker could not read the Rust LLVM bitcode; restoring matching thin-LTO
resolved that tooling failure. Production CLI flags were unchanged.

## Scope and remaining work

`index-root-order-2026-10-26.json` retains sources, source/binary hashes, baseline
failures, candidate passes, native proof, test logs, all 150 checksums, all 140
allocation/timing/resource observations and host telemetry. Builds completed
before timed measurements began.

This closes the approved direct indexed-assignment rule. It does not close the
separate captured-state reentry bug, unused user-binding materialization costs,
all collection representations, production-size verification, clean-host
comparisons, supported-platform acceptance or Goose/Rust rankings. No maintainer
was contacted.
