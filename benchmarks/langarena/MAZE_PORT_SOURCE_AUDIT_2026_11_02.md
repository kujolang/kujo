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

## Executed state/path controls

`probes/audit-maze-state.rb` now executes the entire unchanged pinned Rust maze
module, with its original RNG functions, against the unchanged Kujo sources on
the frozen accepted CLI (`3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`).
The harness supplies dimensions directly rather than calling configuration
constructors. This is a correctness oracle, not a competitive Rust build.

Ten cases cover dimensions 5×5, 8×8 and 17×13 at seeds 1/42/99, plus 200×30 at
seed 42. Each records creation and three generation cycles, resetting spaces
before subsequent generations. All **40 states agree across VM, interpreter and
JIT opt-in** (120 Kujo state observations). The Rust oracle supplies 40 states.
No native-path coverage or timing claim follows from JIT opt-in alone.

Full cell kinds, interior ordered neighbor lists, RNG state and complete BFS
paths match Rust in every observation. Rust's unused border neighbor storage
(`[(0,0);4]`) is normalized to Kujo's empty border adjacency for comparison;
raw representation equality is not claimed. The differing finish-opening
implementations produced equal cells in these controls; visit/allocation counts
remain unmeasured.

Four cases have different A* paths after each generation, with equal lengths:

| Dimensions | Seed | Rust and Kujo path length | Full paths |
| --- | ---: | ---: | --- |
| 5×5 | 99 | 5 | Different |
| 8×8 | 1 | 11 | Different |
| 8×8 | 42 | 11 | Different |
| 17×13 | 42 | 33 | Different |

For example, 5×5 seed 99 returns flattened cell indices `[6,11,12,13,18]` in
Rust and `[6,11,16,17,18]` in Kujo. Both are valid paths in the same generated
maze. The official test dimensions/seed produce identical complete BFS and A*
paths of length **1,337**. This diagnostic runs generation/search directly;
it does not replace the official benchmark wrapper or its checksum gates.

`maze-state-audit-2026-11-02.json.gz` retains generated oracle/probe sources,
commands, raw outputs, comparison report, runner and upstream license, each
with its SHA-256. The first orchestration attempt completed language executions
but failed aggregation on macOS Ruby's missing `filter_map`; the runner uses
`map.compact` and the successful second run supplies the retained comparison.

Reproduce from the repository root with
`ruby benchmarks/langarena/probes/audit-maze-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT_DIRECTORY`.
Expansion order/counts, disconnected/small-dimension behavior, production
resource checks and Goose execution remain open. Equal paths in these cases do
not prove equal work or complete the three benchmark audits.
