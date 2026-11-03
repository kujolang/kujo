# Binary-tree port: bounded full-state audit

Scope: `Binarytrees::Obj` and `Binarytrees::Arena`. Upstream sources are pinned to
`3c9f84ddaeec834c0631747cb789e8cfd602f993`; Rust and Goose working files match their
exact git blobs. Kujo uses frozen accepted runtime `5fe52fc5`, not the pending
loop-local candidate. No benchmark source or runtime was changed for this audit.

## Verified controls

Sixteen inputs cover depths 0, 1, 2, 4 and 7 with root values -17, 0 and 23,
plus depth10/root0. For every input:

- The unchanged Rust object and arena kernels produce identical complete
  preorder node records: item value, left child index and right child index.
- Kujo's unchanged object and arena kernels match those complete records in
  VM, interpreter and JIT, and agree on the final unsigned32 checksum.
- An independent structural check verifies every expected child position and
  item value, node count `2^(depth+1)-1`, and the modular sum of all item+1 values.

The oracle extracts the complete pinned Rust file, removing only its two imports
and supplying unused benchmark-interface/config stubs. Tree construction and
sum kernels are unchanged. An inspection helper serializes the actual Rust
pointer tree; a separate Kujo inspection helper serializes its struct tree.
Arena records are compared directly. This is output/state evidence, not an
instrumented count of construction, traversal, allocation or copy operations.

## Source correspondence and differences

All three implementations construct a full binary tree. At depth d>0, children
receive item minus/plus `1 << (d-1)`, and both subtrees use depth d-1. Arena nodes
are appended in preorder with -1 leaf links. Traversal visits both existing
children and includes item+1. Kujo applies unsigned32 normalization at the
benchmark accumulation boundary; Rust uses wrapping unsigned32 traversal.
The bounded negative-item controls verify the resulting modular sums.

| Representation | Kujo | Rust | Goose |
| --- | --- | --- | --- |
| Object tree | Recursive struct values with named fields | Individually boxed nodes with optional children | Nodes pushed into an arena-like growable collection; typed references |
| Arena tree | Array of three-element node arrays; recursive value-return updates | Vector of fixed-field nodes; mutation by index | Growable fixed-field node collection; mutation by index |
| Run lifecycle | New root/empty arena each run | New boxed root/new arena vector each run | New node collection/new arena each run |

The label “Obj” does not imply identical allocation strategy across languages.
These layout and update differences must be separated from runtime execution
costs. This audit does not quantify allocation counts or a speed penalty.

## Limits and remaining gates

- Goose source was reviewed, but its complete tree state was not executed.
- Official test depth4 is included as an isolated state control. The official
  warmup/iteration runner was not replaced or retimed by this oracle.
- Production depth20 creates 2,097,151 logical nodes per tree. These controls stop
  at depth10 and do not certify production resources, performance or completion.
- Actual construction/traversal/copy counts, wider numeric boundaries and
  production/Goose execution remain open. Neither ledger row is complete.
- Builds overlapped the controls; no speed or native-coverage claim is made.

## Reproduction and evidence

`ruby benchmarks/langarena/probes/audit-tree-state.rb PINNED_ARENA FROZEN_KUJO NEW_OUTPUT`

The final report records source/binary hashes, toolchain, Rust build argv,
complete expected state and all three Kujo process receipts. An earlier attempt
passed the value checks but recorded the last Kujo argv as build argv; its local
outputs remain separate. The corrected collector was rerun and its build argv
and every source/raw-output identity independently checked.

Portable `tree-state-audit-2026-11-03.json.gz` retains source snapshots, generated
oracle/contracts, raw outputs and the final report, with every embedded hash
verified. Executables remain local. Local final evidence:
`.audit-evidence/native-performance-campaign-2026-11-03/tree-state-verified/`.
