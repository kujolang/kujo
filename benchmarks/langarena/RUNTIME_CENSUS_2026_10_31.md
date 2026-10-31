# VM/JIT execution census and kernel samples

Accepted runtime `5a0632d`, CLI SHA-256
`6fa8f819d3bf023bd586996546e2d51735515a4ee846b22c0be2e52b22b8b92c`.
The accepted CLI and frozen runtime/test source manifest were verified before
collection. No runtime implementation changed.

All **100 test-size checksum runs** pass: 50 VM and 50 JIT. Exact coverage, argv,
source/binary identities, measurement receipts and the full counter inventory
pass independent verification. All six subsequent priority sampling runs also
pass their checksums and sampling checks. Formatting and diff checks pass.

## What the counters establish

Every JIT case records native function entries. QuickSort records 900,508;
MergeSort 999,808; NeuralNet 156,010. Their remaining VM call-opcode counts in JIT
mode are 366, 370 and 102 respectively. This rules out complete absence of JIT
execution in these runs. It does not establish that their whole kernels execute
natively, nor that native execution is efficient.

Counters cover process setup, warmup, benchmark execution and checksums. Native
entries, VM call opcodes and JIT call dispatches have different boundaries;
**do not divide them to claim CPU-time coverage or a fallback percentage**.
Cases have different work counts, so the table is not a performance ranking.
Full VM-mode counters and diagnostic timing/resource fields remain in the bundle.

| Benchmark | JIT native entries | VM call opcodes in JIT mode | JIT call dispatches |
| --- | ---: | ---: | ---: |
| Binarytrees::Obj | 50 | 309 | 276 |
| Binarytrees::Arena | 50 | 506 | 276 |
| Brainfuck::Array | 42 | 95 | 204 |
| Brainfuck::Recursion | 72 | 106 | 236 |
| Matmul::Single | 330 | 292 | 412 |
| Matmul::T4 | 358 | 407 | 507 |
| Matmul::T8 | 390 | 511 | 603 |
| Matmul::T16 | 438 | 711 | 779 |
| Base64::Encode | 19497 | 10277 | 19714 |
| Base64::Decode | 19497 | 10278 | 19714 |
| Json::Generate | 5 | 104 | 71 |
| Json::ParseDom | 32 | 256 | 224 |
| Json::ParseMapping | 32 | 262 | 229 |
| CSV::Parse | 843 | 229 | 1017 |
| Etc::Sieve | 8 | 51 | 31 |
| Etc::TextRaytracer | 1109608 | 2427 | 1110423 |
| Etc::NeuralNet | 156010 | 102 | 156082 |
| Etc::CacheSimulation | 6193 | 351 | 6511 |
| Etc::GameOfLife | 611 | 245 | 826 |
| Etc::Words | 3537 | 263 | 3670 |
| Etc::LogParser | 5 | 91 | 68 |
| Template::Regex | 1106 | 603 | 1448 |
| Template::Parse | 949 | 6912 | 1162 |
| Sort::Quick | 900508 | 366 | 900732 |
| Sort::Merge | 999808 | 370 | 1000033 |
| Sort::Self | 99908 | 268 | 100034 |
| Graph::BFS | 118 | 355 | 341 |
| Graph::DFS | 118 | 355 | 341 |
| Graph::AStar | 506 | 355 | 729 |
| Hash::SHA256 | 8818 | 343 | 9033 |
| Hash::CRC32 | 907 | 250 | 1029 |
| Calculator::Ast | 4176 | 376 | 4516 |
| Calculator::Interpreter | 25393 | 2111 | 27017 |
| Maze::Generator | 194699 | 348 | 195019 |
| Maze::BFS | 131504 | 251 | 131726 |
| Maze::AStar | 171704 | 644 | 172121 |
| CLBG::Fannkuchredux | 1359 | 156 | 1583 |
| CLBG::Mandelbrot | 423 | 146 | 540 |
| CLBG::Nbody | 8 | 86 | 57 |
| CLBG::Spectralnorm | 97153 | 142 | 97318 |
| Compress::BWTEncode | 50 | 4336 | 4265 |
| Compress::BWTDecode | 38 | 1104 | 1083 |
| Compress::HuffEncode | 355 | 690 | 689 |
| Compress::HuffDecode | 374 | 770 | 780 |
| Compress::ArithEncode | 1057 | 560 | 1481 |
| Compress::ArithDecode | 3548 | 953 | 4066 |
| Compress::LZWEncode | 1586 | 459 | 1909 |
| Compress::LZWDecode | 256 | 461 | 580 |
| Distance::Jaro | 567 | 298 | 934 |
| Distance::NGram | 7268 | 643 | 7587 |

## Post-warmup sampling

QuickSort, MergeSort and NeuralNet each run in VM and JIT. A macOS sampler starts
when the unchanged port emits `@profile warmup-done`, using a three-second window
at two-millisecond intervals. All six sampler commands succeed and retain call
graphs bound to their target PID and binary. Short runs can produce partial windows.

The release profile strips symbols. Samples therefore contain unknown Kujo frames;
visible system frames include memory copying and environment lookup. This is a
lead for attribution, not proof that a particular debug flag or runtime function
is the cause. Collapsed samples also include waiting threads: their large wait
counts must not be interpreted as percentages of CPU spent waiting.

The next gate is a symbolized diagnostic linked against the accepted runtime,
followed by an isolated reproducer for any proposed change. A separately linked
probe has its own artifact identity and must not be treated as the shipped CLI
for timing comparisons. Do not cache environment flags merely because `getenv`
appears in samples: first establish the caller and preserve observable changes.

## Remaining requirements

This refresh completes process event-count evidence, not section 5. Precise
native/fallback CPU attribution, supported-operation coverage, helper/guard costs,
compilation and cache tradeoffs remain unproved. Collection/snapshot costs from
sections 3/4 also remain where not covered by their accepted fixes. Production-size
completion, platform gates and clean-host Goose/Rust comparisons are separate.
The busy local machine's timings are diagnostic only.

[Evidence bundle](runtime-census-2026-10-31.json.gz): 118 digest-verified artifacts,
including all observations, raw samples, source/binary bindings, collection tools
and verifiers. Run the Ruby collectors from the repository root with the accepted
CLI path and a fresh output directory; see their argument checks. No Kostya contact,
release publication or benchmark algorithm changes occurred.
