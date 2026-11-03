# Game of Life port state audit — partial

The unchanged constructor, generation kernel and checksum pass **540 full-grid
observations** across VM, interpreter and JIT. Every cell's alive flag,
next-state flag and ordered list of eight neighbor indices is checked, along
with the final checksum. No runtime or port implementation changed.

The 45 cases combine nine dimensions—1×1, 1×3, 3×1, 2×2, 2×3, 3×2, 5×5,
7×4 and 20×20—with five seeded patterns: empty, full, checkerboard, an
edge-crossing glider and a block. The initial grid and three subsequent
generations are observed in each runtime. Twenty by twenty is the registered
test-size shape; these patterns do not substitute for its random preparation.

Small dimensions deliberately produce repeated wrapped neighbor indices,
including references to the same cell. The oracle retains all eight offset
occurrences rather than deduplicating them. This verifies toroidal edge behavior
and the two-phase update rule, where every next state uses the prior generation.
An initial snapshot is retained, and its complete cell state must remain
unchanged after every generation.

The collector's reference simulation gathers live values from expected neighbor
lists. An independent verifier instead scatters contributions from each live
source cell into a fresh count grid, then checks every output field and checksum.
Both verify case coverage and raw-output/source identities; matching checksums
alone are insufficient.

## Source and representation boundaries

Pinned Rust and Goose source both use eight wrapped offsets and separate compute
and update passes. Their representations differ: Rust uses nested cell vectors
and coordinate-pair neighbors; Goose's cells store neighbor references; Kujo
uses a flat cell array and arrays of integer neighbor indices. This source review
does not measure allocation, access or copying costs, nor establish equivalent
machine-level work.

Remaining gates include random preparation, invalid/zero dimensions, larger and
production grids, executed Rust/Goose full-state controls, work counts and memory
attribution. The benchmark's structural-equivalence ledger row remains partial.

## Evidence

- Comparator files match upstream
  `3c9f84ddaeec834c0631747cb789e8cfd602f993`; they were inspected, not executed.
- The accepted loop-fix CLI hash is
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  The separate memory candidate is not used. Runs overlap the no-JIT build and
  provide no timing or native-participation claim.
- `probes/audit-life-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces the
  controls. `life-state-audit-2026-11-03.json.gz` retains the fixtures, complete
  outputs, both verifiers, source identities and pinned comparator sources.
