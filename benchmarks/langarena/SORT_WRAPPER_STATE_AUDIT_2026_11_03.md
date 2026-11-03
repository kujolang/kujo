# Sorting wrapper state audit — partial

All three unchanged sorting paths pass **1,764 complete observations** across
VM, interpreter and JIT on the accepted loop-fix runtime. This refreshes the
historical Quick/Merge evidence obtained with a subsequently rejected call
candidate and adds the builtin Sort::Self path. No runtime or port changed.

Coverage combines 49 nonempty integer arrays, three sorting paths, two initial
checksum seeds, two consecutive iterations and three runtimes. Inputs include
lengths 1, 2, 3, 31, 64 and 257; ascending, descending, equal, alternating,
absolute-value-ordered and three deterministic duplicate-heavy patterns; and a
separate signed 32-bit extrema case.

The unchanged wrapper executes alongside a diagnostic clone that exposes the
sorted array only after computing its checksum. RNG state is restored before
the diagnostic call; both calls must produce identical RNG and checksum state.
Complete original arrays and retained snapshots must remain unchanged. The
collector compares full sorted arrays, input arrays, RNG state and checksums.
An independent verifier checks monotonicity and multiplicities without sorting,
uses exact integer arithmetic for RNG-selected indices, and checks wrapped
32-bit checksum accumulation and exact case coverage.

## Boundaries

Pinned Rust and Goose sources are retained, but neither comparator is executed
in this audit. Rust Sort::Self uses `sort()` and Goose uses `sort()`; this does
not establish algorithm identity, equal comparison counts or equal copying.
The wrapper uses crafted arrays: random preparation, empty-input behavior,
distinguishable equal-object stability, operation/call/copy counts, production
sizes and executed comparator controls remain open. All three ledger rows remain
partial. No elapsed-speed or actual native-participation claim follows.

## Evidence

- Comparator source revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Accepted CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  This is not the separate memory candidate. Runs overlapped its no-JIT build.
- `probes/audit-sort-wrapper-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT`
  reproduces the collector; the archive includes the independent verifier.
- `sort-wrapper-state-audit-2026-11-03.json.gz` contains 23 hash-verified
  artifacts: source identities, fixtures, full raw outputs, collector, independent
  verifier and pinned comparator sources. The executable is identified by hash,
  not embedded.
