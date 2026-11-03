# CacheSimulation complete-state audit

The accepted loop-fix CLI verifies **36,000 request-level states and 36 final
checksums** across VM, interpreter and JIT. This is correctness evidence, not a
speed result or completion of the port-equivalence gate.

## Coverage and independent oracle

Three capacity/key-domain pairs `(1,1)`, `(2,5)`, `(8,17)` cover repeated hits,
single-entry state, eviction, reinsertion and multi-node movement. Each uses RNG
seeds 42 and 139967 and two unchanged 1,000-request iterations. The diagnostic
copy differs from `cache_run` only by its name/observer argument and an observation
after each completed request. The original function is also run from the same
initial state/RNG; final state and RNG must match the instrumented copy. Retained
node snapshots must survive both calls.

Every observation includes RNG, head/tail, hit/miss counts and all nodes in LRU
order, including values and both links. Traversal must terminate and visit exactly
the number of dictionary nodes. The independent Ruby verifier uses a simple
ordered list of key/value pairs rather than reproducing linked-node mutation.
It derives RNG keys with integer arithmetic and checks exact row order, coverage,
wrapped checksums, source identities, exit status and raw-output hashes. A negative
control changes the first hit count and refreshes its output hash; the oracle
still rejects the altered state.

The first fixture attempted JSON serialization of a struct-containing snapshot;
VM rejected that unsupported conversion after the first iteration. The corrected
fixture compares explicit node fields. Original failure output, source and exact
collector identity are retained; this was a fixture correction, not a runtime fix.

## Comparator differences and remaining gates

Pinned Rust uses integer node indices in a vector and a reusable free-slot list.
Pinned Goose uses node references, reusable nodes and separately managed string
storage. Kujo uses string-key links in a dictionary of value-semantic structs,
without an explicit free-node pool. All three source implementations move a hit
to the front on lookup and again on update (the second movement can return early).
These layouts do not establish equal allocation, lookup or copying work.

Neither Rust nor Goose is executed by this audit. Production configuration,
zero/invalid capacity, broader key domains/seeds, operation/allocation counts,
executed comparator state and full production behavior remain open. No native
participation or timing claim follows. Runs overlap the separate memory candidate's
production work and cannot serve as performance samples.

## Evidence

- Upstream revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993`.
- Accepted CLI SHA-256:
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  This is not the unaccepted memory candidate.
- `probes/audit-cache-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` collects outcomes;
  `probes/verify-cache-state.rb OUTPUT` independently verifies them.
- `cache-state-audit-2026-11-03.json.gz` preserves raw outcomes, identities,
  fixtures, both collectors, verifier, negative control and pinned comparator
  sources; binaries are identified by hash rather than embedded.
