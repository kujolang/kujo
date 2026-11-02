# Maze port source audit

Scope: `Maze::Generator`, `Maze::BFS` and `Maze::AStar`. This is source review,
not full-state differential execution or a new speed result. No benchmark or
runtime implementation changed. Comparator files match LangArena commit
`3c9f84ddaeec834c0631747cb789e8cfd602f993` with no working-tree diff.

Sources: Kujo `src/80_maze.kujo` and `src/40_graph.kujo` in this directory;
upstream `rust/src/benchmarks/maze.rs` and `goose/src/maze.goose` at that commit.

| Workload | Matching source structure | Differences requiring separate attribution |
| --- | --- | --- |
| Generator | Same neighbor construction order, four random index-pair swaps per interior cell, stack digging rule, finish opening, reset and FNV-style checksum | Rust preallocates its dig stack and collects wall neighbors before recursive finish opening; Kujo/Goose check each neighbor immediately before recursion. Cell storage and ownership differ. |
| BFS | Same neighbor iteration, visited-on-enqueue policy, parent-chain reconstruction and midpoint checksum for a nonempty path | Rust uses `VecDeque` and coordinate tuples; Goose uses a vector plus head and references; Kujo uses `Queue` and value arrays. Rust/Goose explicitly return zero midpoint contribution for an empty path; Kujo directly indexes it. |
| AStar | Same Manhattan heuristic, score arrays, relaxation condition and parent-chain reconstruction | Kujo reuses its graph heap, whose pop policy differs from Rust/Goose; ties are also handled differently. See the verified graph audit. An equal checksum does not prove equal expansion order or work. |

The heap difference is established in
[the graph audit](GRAPH_ORDER_AUDIT_2026_11_02.md). It does **not** by itself prove
that official maze paths, checksums or expansion counts differ. A* normally
relies on priority ordering; full maze traces are required before calling the
ports equivalent or attributing a measured difference solely to the runtime.

Rust/Goose clamp width and height to at least five. Kujo reads the configuration
dimensions directly. This is an out-of-official-input behavior difference, not
evidence that the official cases fail. Likewise, empty-path handling is a
source-visible robustness difference; this audit does not claim that the
official generated mazes produce empty paths.

Official test dimensions are 200 by 30 with two measured iterations for each
case. Production dimensions are 1,400 by 1,400, with ten generator iterations
and seventy search iterations. These remain unchanged. Existing all-50 checksum
passes validate the selected official outputs, not every cell or path element.

## Remaining verification

- Compare every ordered neighbor list and generated cell kind, including RNG
  state, reset, warmup and repeated generation, on small seeds and official input.
- Execute the pinned Rust generation/search methods and unchanged Kujo methods;
  source inspection and a translated model are insufficient as the only oracle.
- Record complete BFS/A* paths, queue expansion order and operation counts.
  Distinguish equal path length from equal path and equal work.
- Isolate dynamic versus precollected finish-neighbor checks with a small control;
  determine whether they change cells, repeated visits, allocation or none of
  those on official input. No effect is assumed here.
- Add disconnected, equal-priority and small-dimension diagnostic controls,
  clearly separate from official suite scoring. Verify all Kujo runtimes.
- Run production structural/resource checks under declared limits. Goose has
  source review only here; execution remains pending.

These three workloads remain **partially audited**, not complete. Any diagnostic
port variant must stay separate from runtime optimization measurements and must
not silently replace the frozen official inputs.
