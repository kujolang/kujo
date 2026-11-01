# Execution census after reusable loops, 2026-11-01

Accepted runtime source `348a6270`, integrated in `0523c2dc`; binary SHA-256 `f5393d6847fec1fb805731258cf57318aa8889a7997e4c60ed45b7d3de3eac11`.
All 100 unchanged test-size VM/JIT cases pass their checksums, exact metric inventory and independent verifier. No runtime or workload change is included.

These counters include setup, warmup, execution and checksums. Native boundaries and native function entries have different meanings; neither establishes native CPU-time coverage. Compile entries count attempts, not necessarily successful unique artifacts. Do not divide these counts to infer a fallback percentage or rank different benchmarks. Wall/CPU/RSS observations are retained as loaded-laptop diagnostics only.

| Benchmark | VM calls in JIT mode | Native boundaries | Native function entries | Compilation attempts | Cache hits |
|---|---:|---:|---:|---:|---:|
| Binarytrees::Obj | 309 | 50 | 50 | 4 | 0 |
| Binarytrees::Arena | 506 | 50 | 50 | 3 | 0 |
| Brainfuck::Array | 95 | 42 | 42 | 5 | 0 |
| Brainfuck::Recursion | 106 | 72 | 72 | 5 | 0 |
| Matmul::Single | 292 | 437 | 330 | 9 | 104 |
| Matmul::T4 | 407 | 472 | 358 | 41 | 110 |
| Matmul::T8 | 511 | 513 | 390 | 73 | 119 |
| Matmul::T16 | 711 | 576 | 438 | 137 | 134 |
| Base64::Encode | 10277 | 19549 | 19497 | 6 | 50 |
| Base64::Decode | 10278 | 19549 | 19497 | 6 | 50 |
| Json::Generate | 104 | 5 | 5 | 3 | 0 |
| Json::ParseDom | 256 | 32 | 32 | 5 | 0 |
| Json::ParseMapping | 262 | 32 | 32 | 5 | 0 |
| CSV::Parse | 229 | 843 | 843 | 5 | 0 |
| Etc::Sieve | 51 | 8 | 8 | 3 | 0 |
| Etc::TextRaytracer | 2427 | 1109608 | 1109608 | 10 | 0 |
| Etc::NeuralNet | 102 | 156010 | 156010 | 8 | 0 |
| Etc::CacheSimulation | 351 | 6193 | 6193 | 4 | 0 |
| Etc::GameOfLife | 245 | 611 | 611 | 7 | 0 |
| Etc::Words | 263 | 3537 | 3537 | 5 | 0 |
| Etc::LogParser | 91 | 5 | 5 | 4 | 0 |
| Template::Regex | 603 | 1106 | 1106 | 4 | 0 |
| Template::Parse | 4859 | 1033 | 949 | 6 | 82 |
| Sort::Quick | 366 | 900508 | 900508 | 5 | 0 |
| Sort::Merge | 370 | 999808 | 999808 | 6 | 0 |
| Sort::Self | 268 | 99908 | 99908 | 3 | 0 |
| Graph::BFS | 355 | 118 | 118 | 6 | 0 |
| Graph::DFS | 355 | 118 | 118 | 6 | 0 |
| Graph::AStar | 355 | 506 | 506 | 8 | 0 |
| Hash::SHA256 | 343 | 8818 | 8818 | 6 | 0 |
| Hash::CRC32 | 250 | 907 | 907 | 5 | 0 |
| Calculator::Ast | 376 | 4176 | 4176 | 12 | 0 |
| Calculator::Interpreter | 2111 | 25394 | 25393 | 16 | 0 |
| Maze::Generator | 348 | 194699 | 194699 | 11 | 0 |
| Maze::BFS | 251 | 131504 | 131504 | 12 | 0 |
| Maze::AStar | 644 | 171704 | 171704 | 16 | 0 |
| CLBG::Fannkuchredux | 156 | 1359 | 1359 | 5 | 0 |
| CLBG::Mandelbrot | 146 | 423 | 423 | 4 | 0 |
| CLBG::Nbody | 86 | 8 | 8 | 5 | 0 |
| CLBG::Spectralnorm | 142 | 97153 | 97153 | 6 | 0 |
| Compress::BWTEncode | 4336 | 111 | 50 | 10 | 54 |
| Compress::BWTDecode | 1104 | 49 | 38 | 7 | 8 |
| Compress::HuffEncode | 690 | 359 | 355 | 10 | 0 |
| Compress::HuffDecode | 770 | 379 | 374 | 13 | 0 |
| Compress::ArithEncode | 560 | 1057 | 1057 | 9 | 0 |
| Compress::ArithDecode | 953 | 3548 | 3548 | 11 | 0 |
| Compress::LZWEncode | 459 | 1586 | 1586 | 6 | 0 |
| Compress::LZWDecode | 461 | 256 | 256 | 7 | 0 |
| Distance::Jaro | 298 | 567 | 567 | 8 | 0 |
| Distance::NGram | 643 | 7268 | 7268 | 7 | 0 |

## Next gate and limits

This refresh closes the event-count refresh for the new revision, not precise native/fallback CPU attribution. Symbolized post-warmup profiles on the new source remain necessary before selecting the next hot-path fix. Compilation time, cache memory/lifetimes, broader compiled-operation support, production coverage and clean-host comparisons remain open. Historical samples and older census counts are not same-revision causal controls. The sections 3/4 bounded cutoff remains complete.

[Evidence bundle](accepted-loop-census-2026-11-01.json.gz): 105 hash-verified artifacts, including all raw measurements, source bindings and collector/verifier. Reproduce with the checked-in census collector and a fresh output directory; verify using `ruby benchmarks/langarena/probes/verify-runtime-census.rb OUTPUT_DIRECTORY`.
