# Execution census after checked validation, 2026-11-01

Accepted runtime `782c712d`; frozen binary SHA-256
`3d6b3ea8e621d6bf8f74ce2cd8f102bc2da0629221b6d3fdc091640ff4e790b3`. All 100 unchanged VM/JIT test-size checksum runs and
exact metric inventories pass independent verification. All 1,390 source,
test, build and vendor files match the accepted candidate manifest.

Native-entry, compilation-attempt, cache-hit and call counts match the previous
`348a6270` census. The 24 changed counters are scheduler rounds, VM entries and
promise polls in threaded Matmul T4/T8/T16; their scheduling-dependent counts
are retained in the comparison receipt. No runtime change accompanies this census.

Counts include setup, warmup, execution and checksums. Native boundaries and
native function entries are different events; neither measures native CPU time.
Compilation entries count attempts, not unique successful artifacts. Timing,
CPU and peak RSS remain loaded-laptop diagnostics, not comparative scores.

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

## Remaining evidence

This refresh completes current-revision event counting, not precise native/fallback
CPU attribution. Compile-time, code-memory and cache-lifetime investigation remains
open. Inspection of pinned Cranelift 0.113.1 shows its memory allocator intentionally
retains executable mappings on drop; Kujo does not invoke its explicit release
method. The next investigation must measure compiler/VM lifecycle retention and
prove compiled-pointer ownership before proposing reclamation. No memory fix or
measured growth claim is made by this source observation.

[106-artifact evidence bundle](accepted-validation-census-2026-11-01.json.gz)
contains raw measurements, checksums, source bindings, comparison and verifier.
The census process completed successfully. Broader section 5, production and
clean-host competitive acceptance remain open.
