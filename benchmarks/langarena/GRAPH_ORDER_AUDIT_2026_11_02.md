# Graph ordering audit — 2026-11-02

This audit advances the all-50 work-equivalence review. It does not change the
port or runtime, establish production behavior, or measure speed. It runs against
frozen accepted Kujo `782c712d`, binary SHA-256
`3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`.
The runtime array-region candidate is separate and remains unaccepted.

## Finding

The Kujo graph priority queue reproduces pinned **Crystal** heap behavior, not
Rust/Goose heap ordering. All sources were checked against LangArena commit
`3c9f84ddaeec834c0631747cb789e8cfd602f993`.

- Kujo `graph_heap_pop` and Crystal `Graph::AStar::PriorityQueue#pop` compare
  children against the old root before putting the last item into the hole.
  Pushing vertices `0,1,2,3` with priorities `1,2,3,4` then popping all entries
  yields `0,3,1,2`, rather than priority order `0,1,2,3`.
- Rust uses `BinaryHeap<Reverse<Node>>` with priority followed by vertex order.
  Goose uses its heap helpers with the same priority/vertex comparator. Kujo's
  push compares priority alone, so ties also have a different contract.
- The retained Kujo control confirms the non-priority pop sequence in VM,
  interpreter and JIT CLI modes. It describes port behavior; it is not a defect
  in Kujo's general collection or runtime machinery.

## Graph controls and limits

A labeled Ruby model compares Kujo's exact queue policy with a queue sorted by
the Rust/Goose comparator. Both use the same A* relaxation and heuristic.
Fixed-seed small undirected graphs produce three differing expansion traces;
one produces return values **2 versus 3**. The Kujo return values are independently
asserted by executing the unchanged graph functions in all three engines.
These graphs deliberately cover shapes outside the official generator's chain
backbone. A subsequent standalone oracle executes the unchanged pinned Rust
`heuristic` and `a_star_shortest_path` methods plus their original `Node` ordering.
It confirms all three model results: `2,2,3`, versus Kujo's `2,2,2`. Goose remains
source-reviewed only. The oracle is a correctness check, not a competitive binary.

The official `test.json` graph is checked separately. Full ordered adjacency
generated from seed 42 matches the model in all three Kujo modes, and both queue
models produce the same ten-entry expansion trace and result **9**; the executed
Rust oracle independently returns **9**. Thus the
official checksum can pass while the queue implementations differ. This audit
does not claim an observed official-test result or traversal mismatch.

The heuristic `target - vertex` is not generally admissible when long jumps cost
one edge. Neither modeled A* result is asserted to be the mathematical shortest
path for arbitrary graphs. The purpose is work/behavior comparison, not silently
repairing the benchmark algorithm.

## Other graph work differences

Source review finds matching BFS/DFS traversal structure and adjacency insertion
order. Storage differs: Rust uses typed tuple queues/stacks and `Vec<bool>`;
Goose uses typed `Step` records, boolean storage and an append-only BFS queue with
a head index; Kujo uses generic pair arrays, byte visitation storage and its
persistent `Queue` API. Graph preparation uses Rust per-row vectors, Goose an
edge arena with resized slices, and Kujo nested value arrays. These mechanisms
do not establish allocation counts or elapsed-time attribution.

Production inputs have 500,000 vertices, 20 possible jumps and jump length 1,000.
Their full traces and production competitor executions remain pending. Do not use
the tiny control to estimate production heap cost or to change official inputs.

## Disposition and reproduction

Keep the existing port unchanged while collecting comparator-specific evidence.
If a Rust/Goose-equivalent heap variant is later measured, label it as a port
variant, preserve the Crystal-based baseline, and keep that result separate from
runtime improvements. No maintainer contact occurred.

`probes/audit-graph-order.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT_DIRECTORY` verifies
pinned source identity, emits reproducible full controls, runs three modes and
records source/binary hashes and raw output. Ruby is repository-maintenance glue;
all checked language behavior runs in Kujo. The initial model-only attempt used
an incorrectly predicted pop sequence and was corrected before runtime validation;
a chain-backbone-only search found no differing traces and motivated the explicitly
labeled broader graph controls.

`graph-order-audit-2026-11-02.json.gz` preserves the runner, generated fixture and
three-mode report, plus the Rust oracle source, build log, report and runner,
with per-file SHA-256 checks. The Rust report binds its original input report,
pinned source, compiler version and binary hash. Run
`probes/verify-graph-rust.rb UPSTREAM GRAPH_REPORT NEW_OUTPUT_DIRECTORY` to repeat
the four Rust controls after other builds stop. These checks are separate from
official all-50 checksum and performance gates.
