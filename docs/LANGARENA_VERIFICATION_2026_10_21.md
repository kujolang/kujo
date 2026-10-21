# LangArena verification checkpoint — 2026-10-21

## What was actually verified

- Kujo revision: `4113d0dc9fd704620a37ec8cc451d24701934afb`, macOS x86-64 debug binary.
- Upstream revision: `3c9f84ddaeec834c0631747cb789e8cfd602f993` (fresh clone and remote HEAD verification).
- Upstream Python 3.10.5 reference: **50/50 test.js checksums passed**, exit 0. This is reference-fixture validation, **not a Kujo result**.
- Kujo prerequisite regression gate: **101 passed**, 2 opt-in resource/profile tests ignored. This covers assignments, numeric/CSV/regex/sort APIs, bytes, collections, helper/import ownership, recursion, struct storage, tasks and exception handling. Runtime tests generally exercise both VM and interpreter; not every Rust test represents two language executions.
- Kujo benchmark checksums: **0/50 verified; all 50 blocked by the absent Kujo benchmark implementations and runner**. No run.js benchmark was executed in Kujo. No failed benchmark checksum is being inferred from that absence.
- Existing depth-20 tree and 3 MiB byte probes are prerequisite evidence only; see [collection profiling](COLLECTION_PROFILING.md). They do not establish full-suite throughput, memory or checksum parity.

Input fingerprints (SHA-256):

- test.js: `424371dd25f3fa3f779789c823a05dbe90d2eddbc2a3ca3c5e4319252c572797`
- run.js: `8fe7baed41e8eb0a233057c4406446029f7e5d792cf3a3ac6fc7e714964a155f`

Reproduce the reference baseline: `cd python && python3 main.py ../test.js` at the pinned upstream revision. The original output ended `Summary: 2.9344s, 50, 50, 0`; timing is a local observation, not a Kujo comparison.

## Remaining gaps

1. **Executable coverage, all 50:** no Kujo dispatch registry, benchmark implementations, lifecycle runner or expected-checksum assertions exists yet. A prerequisite regression passing is not an implemented benchmark.
2. **Typed JSON mapping:** current JSON parsing produces dynamic values; direct typed deserialization equivalent to the reference is absent. Explicit DOM-to-struct mapping is implementable, but equivalence/acceptance must be decided before claiming ParseMapping is comparable.
3. **Representation fidelity:** shared mutable graphs in NeuralNet, CacheSimulation, GameOfLife and Maze need idiomatic indexed state under Kujo value semantics. Tree/AST object workloads must preserve fresh object allocation. No new runtime aliasing feature is assumed necessary.
4. **Parallel matrix execution:** demonstrate true 4/8/16-worker execution, correct task admission, transpose placement, transfer cost and worker lifetime. A serial checksum match would not verify these variants.
5. **Full-scale resource completion:** the production configs have not run. Buffer caps, recursion depth, memory growth, scheduler deadlines and release performance remain unknown at these actual workloads. Do not raise limits speculatively or classify debug timeouts as release performance failures.
6. **Integration:** release/native and Linux/Docker runner verification, fail-fast checksum exits, stdout format, filtering, config loading, timing boundaries and upstream formatting are still absent.

No new runtime defect was reproduced by the 101-test prerequisite gate. The list below identifies port/validation work and acceptance risks, not 50 proven runtime bugs.

## Per-benchmark matrix

Every row passed in the **Python reference test configuration** and is **not run in Kujo**. Checksums below are the unmodified upstream test.js expectations. Production parameters are from run.js; warmup is additional.

| Benchmark | Test checksum | Production parameters | Remaining verification / implementation gap |
| --- | ---: | --- | --- |
| Binarytrees::Obj | 124 | depth=20, iterations=30 | Fresh, distinct struct nodes per run; preserve item arithmetic. Depth-20 prerequisite passed previously; full 30 iterations plus warmup are unverified. |
| Binarytrees::Arena | 124 | depth=20, iterations=30 | Index arena distinct from Obj; allocate a new arena each run. Do not replace fresh allocation with clearing/reuse. |
| Brainfuck::Array | 3562897308 | warmup_iterations=1000, iterations=1 | Byte wrapping, tape growth, bracket table and output checksum; use the special warmup program and iteration override. |
| Brainfuck::Recursion | 3562897308 | warmup_iterations=1000, iterations=1 | Preserve recursive operation tree and loop semantics; special warmup; measure actual recursion depth. |
| Matmul::Single | 720656960 | n=900, iterations=10 | Nested rows, transpose, arithmetic order and seven-decimal checksum. Full n=900 is unverified. |
| Matmul::T4 | 2420334760 | n=900, iterations=10 | Four real workers; transpose before dispatch; account for transfer/copy costs and worker lifetime. |
| Matmul::T8 | 4014223764 | n=900, iterations=10 | Eight real workers; same matrix representation and reduction order as Single. |
| Matmul::T16 | 418055336 | n=900, iterations=10 | Sixteen real workers within the 16-task admission cap; coordinator must not consume a seventeenth worker slot. |
| Base64::Encode | 3894061047 | size=500000, iterations=5000 | Fresh output each run; exact UTF-8/byte lengths and accumulated warmup state. Full 500,000-byte payload unverified. |
| Base64::Decode | 732348015 | size=500000, iterations=4000 | Retain byte output and exact decoded length; fresh output per iteration, including warmup. |
| Json::Generate | 4 | coords=20000, iterations=100 | Preserve nested opts dictionaries/arrays, generation order and checksum behavior; structs cannot be passed directly to to_json. |
| Json::ParseDom | 2918383076 | coords=10000, iterations=100 | DOM parsing and seven-decimal coordinate-average checksum; preserve generated fixture/RNG order. |
| Json::ParseMapping | 1969907348 | coords=25000, iterations=100 | No direct JSON-to-typed-struct decoder in current builtin contract. Implement explicit mapping or add a decoder; maintainer acceptance remains open. |
| CSV::Parse | 3563398641 | rows=100000, iterations=70 | parse_csv_rows preserves strings; perform specified numeric conversions and quoting/newline handling. Full input budget must be measured. |
| Etc::Sieve | 4660 | limit=3000000, iterations=270 | Three-million-cell full buffer exceeds the one-million-element range constructor cap; allocate in valid chunks/loops without changing sieve algorithm. |
| Etc::TextRaytracer | 1438660 | w=800, h=800, iterations=100 | Preserve vector/sphere/light math, output characters and floating-point evaluation order; full 800x800 unverified. |
| Etc::NeuralNet | 1586674334 | iterations=1700 | Shared mutable neuron/synapse graph needs explicit indexed state under value semantics; preserve update order and justify representation. |
| Etc::CacheSimulation | 179773224 | values=5000, size=2500, iterations=5000 | Real allocated string keys, map and linked-LRU behavior; no fixed-key simplification. Indexed links need idiomatic-representation explanation. |
| Etc::GameOfLife | 4006235749 | w=500, h=500, iterations=500 | Preserve neighbour relationships and simultaneous generation updates; use explicit indexed state with snapshot-safe ownership. |
| Etc::Words | 580539813 | words=50000, word_len=4, iterations=360 | Exact generated words, dictionary counts, ordering and checksum; preserve RNG consumption. |
| Etc::LogParser | 117 | iterations=10, lines_count=52000 | Exact 13 regex patterns and non-overlapping counts; regex_count/cache prerequisites pass, whole workload unverified. |
| Template::Regex | 1554169905 | iterations=50, count=50000 | Regex callback substitution with exact missing-key behavior and fresh output; verify full output against 8 MiB API limit. |
| Template::Parse | 917360148 | iterations=50, count=50000 | Manual parser must remain distinct from Regex; exact placeholder/literal handling and fresh output. |
| Sort::Quick | 2548773 | size=400000, iterations=50 | Implement specified quicksort, not builtin sort; fresh working copy, duplicates, pivot and recursion behavior. |
| Sort::Merge | 2548773 | size=400000, iterations=50 | Implement specified merge sort and temporary buffers; no substitution with builtin sort. |
| Sort::Self | 2548773 | size=400000, iterations=50 | Use native sort as intended; preserve input copies and checksum. Full 400,000-element run unverified. |
| Graph::BFS | 20 | vertices=500000, jumps=20, jump_len=1000, iterations=40 | Exact seeded graph and neighbour order, FIFO traversal and path result; full 500,000 vertices unverified. |
| Graph::DFS | 28 | vertices=500000, jumps=20, jump_len=1000, iterations=15 | Exact graph and LIFO visitation order; retain explicit stack semantics rather than arbitrary recursive substitution. |
| Graph::AStar | 36 | vertices=500000, jumps=20, jump_len=1000, iterations=1000 | Exact heuristic, heap ordering/tie behavior and path cost; queue/array primitives alone do not verify A*. |
| Hash::SHA256 | 1036619072 | size=1000000, iterations=1200 | Port the reference SimpleSHA256 toy algorithm, NOT the native cryptographic SHA-256 builtin; uint32 wrapping and result byte order matter. |
| Hash::CRC32 | 3031698216 | size=1000000, iterations=150 | Port reference per-byte/per-bit loop, unsigned shifts and uint32 masking; do not replace with a different work-reducing builtin. |
| Calculator::Ast | 774028720 | operations=11000, iterations=100 | Preserve parse tree objects, parser grammar, expression retention and allocation lifetime; no silent arena substitution. |
| Calculator::Interpreter | 3208680 | operations=35000, iterations=100 | Evaluate the same AST with wrapping i64 arithmetic and exact division semantics; full expression depth unverified. |
| Maze::Generator | 2309652669 | w=1400, h=1400, iterations=10 | Preserve randomized carving, cell graph, walls and recursive finish opening. Full 1400x1400 depth/memory unverified. |
| Maze::BFS | 6771 | w=1400, h=1400, iterations=70 | Same generated maze, FIFO order, predecessor chain and final path checksum; full-scale state copying unverified. |
| Maze::AStar | 6771 | w=1400, h=1400, iterations=70 | Same maze, heuristic, priority ordering and path reconstruction; indexed graph representation must remain faithful. |
| CLBG::Fannkuchredux | 4428 | n=8, iterations=700 | Exact permutation generation, swaps, sign and flip counting; wrapping/accumulation contract. |
| CLBG::Mandelbrot | 2236342953 | w=400, h=400, iterations=100 | Exact pixel iteration, bit packing, row padding and accumulated byte output; no Unicode-character substitution. |
| CLBG::Nbody | 338796608 | iterations=20000 | Preserve initial constants, offset momentum, step order and seven-decimal energy checksum. |
| CLBG::Spectralnorm | 1052168784 | size=1200, iterations=132 | Preserve matrix-free formula, vector iteration order and checksum; full size=1200 and iteration count unverified. |
| Compress::BWTEncode | 200 | size=100000, iterations=30 | Reference suffix/rank sorting and original index; retain comparator order and byte semantics. |
| Compress::BWTDecode | 100204 | size=1000001, iterations=180 | Reference frequency/occurrence tables and inverse transform; full size 1,000,001 needs allocation beyond one range call. |
| Compress::HuffEncode | 56 | size=1000002, iterations=200 | Preserve frequency tree, heap tie ordering, codes and bit packing; no native compression replacement. |
| Compress::HuffDecode | 100212 | size=1000003, iterations=350 | Use matching tree/code stream and round-trip comparison; full size 1,000,003 unverified. |
| Compress::ArithEncode | 56 | size=100004, iterations=500 | Exact integer interval, renormalization and bit stream; mask unsigned state deliberately. |
| Compress::ArithDecode | 100220 | size=100005, iterations=350 | Exact cumulative-frequency search, interval math and bit input; preserve decoded-byte comparison. |
| Compress::LZWEncode | 208 | size=200006, iterations=85 | Binary-safe dictionary keys, code-width behavior and output bytes; preserve dictionary growth and order. |
| Compress::LZWDecode | 100244 | size=1000007, iterations=1600 | Handle next-code special case, dictionary values and byte concatenation; full size 1,000,007 unverified. |
| Distance::Jaro | 7020 | iterations=780, count=1000, size=50 | Exact match window, matched flags, transposition handling and float checksum; byte inputs are ASCII. |
| Distance::NGram | 42 | iterations=100, count=10, size=20000 | Exact n-gram windows, duplicate frequencies and similarity accumulation; string-key semantics and mutation order. |

## Maintainer questions already anticipated

The [Goose review](https://github.com/kostya/LangArena/pull/28#issuecomment-6099945027) requires consistent benchmark dispatch/lifecycle, bounds-check disclosure, real JSON config parsing, fresh allocations and faithful matrix/tree/graph representations. Its [cache follow-up](https://github.com/kostya/LangArena/pull/28#issuecomment-6234885793) rejects fixed-key simplification. These are concrete comparability constraints, not reasons to redesign Kujo indiscriminately.

Answers that can already be given: Kujo uses checked indexing and value semantics; imported functions currently execute through AST runtime machinery; structs can contain multiple collections; dictionaries own string keys; fresh state can be created per run; native sort belongs only in Sort::Self. Questions still requiring an implemented proposal or Kostya's input: ParseMapping equivalence, indexed shared-graph representations, and the exact parallel worker/measurement arrangement. No contact was made.

The reference sequence is instantiate, reset RNG, prepare, warmup, reset RNG, measured iterations, checksum. **Benchmark state survives warmup.** Brainfuck supplies special warmup behavior. UInt32 checksum accumulation wraps; floating checksums format seven decimals. These rules must be validated independently before marking any Kujo row passed.

A local-port-versus-audit scope question is pending with the user. This checkpoint introduces no benchmark implementation, runtime fix or upstream submission.
