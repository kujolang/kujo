# Checkpoint comparison — 2026-11-02

Seven rotating rounds of all 50 unchanged LangArena **test-size** workloads on the same loaded laptop. All 2,100 sample records were independently checked against raw stdout/stderr, checksum outcomes, timing, peak RSS, configuration/order and frozen binary hashes. This is not a production-size result or an official LangArena score.

Checkpoint `7fd4f499` contains accepted runtime `782c712d`; original Kujo is `f037705190bb6ef765238f9c091086eae0e97485`. The pending VM memory candidate is excluded. Full identities and release-check dispositions are in [checkpoint review](CHECKPOINT_REVIEW_2026_11_02.md). No tags, publication or runtime fixes were made during this comparison.

## Correctness and aggregate elapsed time

Suite time is the median of seven per-round sums across these 50 workloads; it weights this particular input mix. Kernel time covers the measured loop (including any JIT compilation triggered inside it), excluding process startup. Process wall time also includes setup, compilation outside that loop and checksum work. Competitor kernel totals sum millisecond-rounded values, including rounded zeros, so they are approximate and unsuitable for precise all-suite speedup ratios. Failed variants receive no suite speed score.

| Variant | Passing samples / 350 | Suite kernel seconds | Suite process wall seconds |
| --- | ---: | ---: | ---: |
| Baseline VM | 350 | 61.146 | 101.364 |
| Baseline JIT | 119 | — | — |
| Latest VM | 350 | 48.675 | 82.307 |
| Latest JIT | 350 | 24.252 | 53.276 |
| Rust | 350 | 0.061 | 1.221 |
| Goose | 350 | 0.049 | 1.250 |

## Changes and competitor gaps

These ratios use the median of seven within-round ratios per benchmark, then an equal-weight geometric mean across the stated coverage. Only benchmarks where both variants passed all seven runs and all timings were nonzero qualify. Zero-rounded Rust/Goose values are excluded, not treated as instantaneous. Lower-time counts are descriptive, not statistical significance claims. Different coverage means different aggregates must not be compared as though they used the same workloads.

| Comparison | Result | Covered benchmarks / 50 | Checkpoint lower time / covered |
| --- | ---: | ---: | ---: |
| Latest JIT speedup over Baseline JIT | 0.819× (above 1 = faster) | 17 | 10 / 17 |
| Latest VM speedup over Baseline VM | 1.203× (above 1 = faster) | 50 | 45 / 50 |
| Latest JIT speedup over Baseline VM | 1.180× (above 1 = faster) | 50 | 35 / 50 |
| Latest VM runtime relative to Rust | 359.615× (above 1 = slower) | 15 | 0 / 15 |
| Latest JIT runtime relative to Rust | 372.538× (above 1 = slower) | 15 | 0 / 15 |
| Latest VM runtime relative to Goose | 521.168× (above 1 = slower) | 9 | 0 / 9 |
| Latest JIT runtime relative to Goose | 381.964× (above 1 = slower) | 9 | 0 / 9 |

Comparing checkpoint JIT with original VM measures the combined evolution and runtime choice; it is not a like-for-like JIT speedup. Original JIT correctness failures prevent an all-suite JIT baseline comparison.

## Review conclusion and regressions

For this weighted test-size suite, checkpoint VM is 1.256× faster than original VM, and checkpoint JIT is 2.521× faster than original VM. That does not establish a blanket JIT-to-JIT improvement. On the 17 workloads original JIT completes correctly, its paired geometric-mean speedup is only 0.819×: the checkpoint is slower on that aggregate.

The proposed checkpoint is suitable for stopping and reviewing accepted correctness/engineering work. It is not evidence of Rust/Goose competitive performance. Completing release gates remains necessary before publication; further speed work is a separate decision, not silently added to this round.

The table below lists cases slower in **every paired round** against the matching original runtime. This supports follow-up, not attribution to a particular change. Small or inconsistent VM slowdowns remain visible in the full JSON and are not hidden.

| Runtime | Workload | Original / checkpoint paired median | Paired range |
| --- | --- | ---: | ---: |
| VM | CLBG::Mandelbrot | 0.906× | 0.900–0.921× |
| JIT | Binarytrees::Obj | 0.070× | 0.060–0.098× |
| JIT | Binarytrees::Arena | 0.038× | 0.036–0.042× |
| JIT | Matmul::T4 | 0.427× | 0.332–0.449× |
| JIT | Matmul::T8 | 0.276× | 0.233–0.292× |
| JIT | Matmul::T16 | 0.160× | 0.150–0.213× |
| JIT | Compress::BWTEncode | 0.916× | 0.851–0.952× |
| JIT | Compress::HuffEncode | 0.240× | 0.228–0.266× |

A ratio below 1 means the checkpoint takes longer. Binary-tree and threaded-matrix small-input JIT results particularly warrant examining compilation/setup and execution separately in a future authorized campaign. This run does not prove their root cause.

## All 50 workload medians

Kernel seconds; `0.000` for competitors means below their millisecond reporting resolution. `FAIL n/7` gives failed samples; successful remnants are not used to score that row. The verified JSON also retains each range, process-wall median, RSS median and every raw-output hash.

| Workload | Original VM | Original JIT | Checkpoint VM | Checkpoint JIT | Rust | Goose |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Binarytrees::Obj | 0.002 | 0.002 | 0.001 | 0.029 | 0.000 | 0.000 |
| Binarytrees::Arena | 0.002 | 0.002 | 0.002 | 0.058 | 0.000 | 0.000 |
| Brainfuck::Array | 0.134 | 0.133 | 0.120 | 0.070 | 0.000 | 0.000 |
| Brainfuck::Recursion | 0.130 | 0.132 | 0.120 | 0.070 | 0.000 | 0.000 |
| Matmul::Single | 0.366 | 0.374 | 0.380 | 0.091 | 0.000 | 0.000 |
| Matmul::T4 | 0.149 | 0.151 | 0.149 | 0.376 | 0.001 | 0.000 |
| Matmul::T8 | 0.150 | 0.144 | 0.144 | 0.512 | 0.001 | 0.000 |
| Matmul::T16 | 0.189 | 0.192 | 0.179 | 1.068 | 0.002 | 0.000 |
| Base64::Encode | 0.220 | FAIL 7/7 | 0.185 | 0.275 | 0.001 | 0.001 |
| Base64::Decode | 0.213 | FAIL 7/7 | 0.173 | 0.258 | 0.001 | 0.001 |
| Json::Generate | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Json::ParseDom | 0.001 | FAIL 7/7 | 0.001 | 0.002 | 0.000 | 0.000 |
| Json::ParseMapping | 0.002 | FAIL 7/7 | 0.001 | 0.002 | 0.000 | 0.000 |
| CSV::Parse | 0.002 | FAIL 7/7 | 0.001 | 0.002 | 0.000 | 0.000 |
| Etc::Sieve | 0.014 | 0.014 | 0.014 | 0.004 | 0.000 | 0.000 |
| Etc::TextRaytracer | 5.863 | FAIL 7/7 | 3.370 | 2.784 | 0.001 | 0.001 |
| Etc::NeuralNet | 9.613 | 10.671 | 7.780 | 4.691 | 0.003 | 0.002 |
| Etc::CacheSimulation | 0.781 | FAIL 7/7 | 0.749 | 0.729 | 0.001 | 0.001 |
| Etc::GameOfLife | 0.046 | FAIL 7/7 | 0.045 | 0.025 | 0.000 | 0.000 |
| Etc::Words | 0.010 | FAIL 7/7 | 0.010 | 0.009 | 0.000 | 0.000 |
| Etc::LogParser | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| Template::Regex | 0.004 | FAIL 7/7 | 0.001 | 0.001 | 0.000 | 0.000 |
| Template::Parse | 0.018 | FAIL 7/7 | 0.017 | 0.017 | 0.000 | 0.000 |
| Sort::Quick | 8.961 | FAIL 7/7 | 8.402 | 2.109 | 0.019 | 0.020 |
| Sort::Merge | 30.083 | FAIL 7/7 | 21.717 | 7.192 | 0.021 | 0.015 |
| Sort::Self | 0.134 | FAIL 7/7 | 0.127 | 0.134 | 0.006 | 0.006 |
| Graph::BFS | 0.005 | FAIL 7/7 | 0.004 | 0.003 | 0.000 | 0.000 |
| Graph::DFS | 0.008 | FAIL 7/7 | 0.009 | 0.006 | 0.000 | 0.000 |
| Graph::AStar | 0.007 | FAIL 7/7 | 0.005 | 0.003 | 0.000 | 0.000 |
| Hash::SHA256 | 0.066 | FAIL 7/7 | 0.058 | 0.043 | 0.000 | 0.000 |
| Hash::CRC32 | 0.186 | FAIL 7/7 | 0.182 | 0.128 | 0.000 | 0.000 |
| Calculator::Ast | 0.035 | FAIL 7/7 | 0.028 | 0.025 | 0.000 | 0.000 |
| Calculator::Interpreter | 0.245 | FAIL 7/7 | 0.182 | 0.241 | 0.001 | 0.000 |
| Maze::Generator | 1.432 | FAIL 7/7 | 1.268 | 0.932 | 0.001 | 0.000 |
| Maze::BFS | 0.322 | FAIL 7/7 | 0.292 | 0.199 | 0.001 | 0.000 |
| Maze::AStar | 0.793 | FAIL 7/7 | 0.675 | 0.447 | 0.001 | 0.001 |
| CLBG::Fannkuchredux | 0.020 | 0.018 | 0.019 | 0.006 | 0.000 | 0.000 |
| CLBG::Mandelbrot | 0.278 | FAIL 7/7 | 0.307 | 0.016 | 0.000 | 0.000 |
| CLBG::Nbody | 0.728 | 0.728 | 0.637 | 0.341 | 0.000 | 0.000 |
| CLBG::Spectralnorm | 0.710 | FAIL 7/7 | 0.596 | 0.292 | 0.000 | 0.000 |
| Compress::BWTEncode | 0.146 | 0.146 | 0.037 | 0.159 | 0.000 | 0.000 |
| Compress::BWTDecode | 0.007 | 0.008 | 0.007 | 0.003 | 0.000 | 0.000 |
| Compress::HuffEncode | 0.015 | 0.016 | 0.013 | 0.065 | 0.000 | 0.000 |
| Compress::HuffDecode | 0.009 | FAIL 7/7 | 0.006 | 0.057 | 0.000 | 0.000 |
| Compress::ArithEncode | 0.019 | FAIL 7/7 | 0.017 | 0.010 | 0.000 | 0.000 |
| Compress::ArithDecode | 0.029 | FAIL 7/7 | 0.025 | 0.016 | 0.000 | 0.000 |
| Compress::LZWEncode | 0.014 | FAIL 7/7 | 0.011 | 0.010 | 0.000 | 0.000 |
| Compress::LZWDecode | 0.006 | FAIL 7/7 | 0.005 | 0.004 | 0.000 | 0.000 |
| Distance::Jaro | 0.003 | FAIL 7/7 | 0.002 | 0.004 | 0.000 | 0.000 |
| Distance::NGram | 0.032 | FAIL 7/7 | 0.026 | 0.023 | 0.000 | 0.000 |

## Ratio exclusions

- **Baseline JIT divided by Latest JIT**: Base64::Encode, Base64::Decode, Json::ParseDom, Json::ParseMapping, CSV::Parse, Etc::TextRaytracer, Etc::CacheSimulation, Etc::GameOfLife, Etc::Words, Template::Regex, Template::Parse, Sort::Quick, Sort::Merge, Sort::Self, Graph::BFS, Graph::DFS, Graph::AStar, Hash::SHA256, Hash::CRC32, Calculator::Ast, Calculator::Interpreter, Maze::Generator, Maze::BFS, Maze::AStar, CLBG::Mandelbrot, CLBG::Spectralnorm, Compress::HuffDecode, Compress::ArithEncode, Compress::ArithDecode, Compress::LZWEncode, Compress::LZWDecode, Distance::Jaro, Distance::NGram.
- **Baseline VM divided by Latest VM**: none.
- **Baseline VM divided by Latest JIT**: none.
- **Rust divided by Latest VM**: Binarytrees::Obj, Binarytrees::Arena, Brainfuck::Array, Brainfuck::Recursion, Matmul::Single, Json::Generate, Json::ParseDom, Json::ParseMapping, CSV::Parse, Etc::Sieve, Etc::GameOfLife, Etc::Words, Etc::LogParser, Template::Regex, Template::Parse, Graph::BFS, Graph::DFS, Graph::AStar, Hash::SHA256, Hash::CRC32, Calculator::Ast, CLBG::Fannkuchredux, CLBG::Mandelbrot, CLBG::Nbody, CLBG::Spectralnorm, Compress::BWTEncode, Compress::BWTDecode, Compress::HuffEncode, Compress::HuffDecode, Compress::ArithEncode, Compress::ArithDecode, Compress::LZWEncode, Compress::LZWDecode, Distance::Jaro, Distance::NGram.
- **Rust divided by Latest JIT**: Binarytrees::Obj, Binarytrees::Arena, Brainfuck::Array, Brainfuck::Recursion, Matmul::Single, Json::Generate, Json::ParseDom, Json::ParseMapping, CSV::Parse, Etc::Sieve, Etc::GameOfLife, Etc::Words, Etc::LogParser, Template::Regex, Template::Parse, Graph::BFS, Graph::DFS, Graph::AStar, Hash::SHA256, Hash::CRC32, Calculator::Ast, CLBG::Fannkuchredux, CLBG::Mandelbrot, CLBG::Nbody, CLBG::Spectralnorm, Compress::BWTEncode, Compress::BWTDecode, Compress::HuffEncode, Compress::HuffDecode, Compress::ArithEncode, Compress::ArithDecode, Compress::LZWEncode, Compress::LZWDecode, Distance::Jaro, Distance::NGram.
- **Goose divided by Latest VM**: Binarytrees::Obj, Binarytrees::Arena, Brainfuck::Array, Brainfuck::Recursion, Matmul::Single, Matmul::T4, Matmul::T8, Matmul::T16, Json::Generate, Json::ParseDom, Json::ParseMapping, CSV::Parse, Etc::Sieve, Etc::GameOfLife, Etc::Words, Etc::LogParser, Template::Regex, Template::Parse, Graph::BFS, Graph::DFS, Graph::AStar, Hash::SHA256, Hash::CRC32, Calculator::Ast, Calculator::Interpreter, Maze::Generator, Maze::BFS, CLBG::Fannkuchredux, CLBG::Mandelbrot, CLBG::Nbody, CLBG::Spectralnorm, Compress::BWTEncode, Compress::BWTDecode, Compress::HuffEncode, Compress::HuffDecode, Compress::ArithEncode, Compress::ArithDecode, Compress::LZWEncode, Compress::LZWDecode, Distance::Jaro, Distance::NGram.
- **Goose divided by Latest JIT**: Binarytrees::Obj, Binarytrees::Arena, Brainfuck::Array, Brainfuck::Recursion, Matmul::Single, Matmul::T4, Matmul::T8, Matmul::T16, Json::Generate, Json::ParseDom, Json::ParseMapping, CSV::Parse, Etc::Sieve, Etc::GameOfLife, Etc::Words, Etc::LogParser, Template::Regex, Template::Parse, Graph::BFS, Graph::DFS, Graph::AStar, Hash::SHA256, Hash::CRC32, Calculator::Ast, Calculator::Interpreter, Maze::Generator, Maze::BFS, CLBG::Fannkuchredux, CLBG::Mandelbrot, CLBG::Nbody, CLBG::Spectralnorm, Compress::BWTEncode, Compress::BWTDecode, Compress::HuffEncode, Compress::HuffDecode, Compress::ArithEncode, Compress::ArithDecode, Compress::LZWEncode, Compress::LZWDecode, Distance::Jaro, Distance::NGram.

## Process memory

Peak resident memory from `/usr/bin/time -l`, converted to MiB, across passing samples only. This measures one process per test-size workload; it is not retained-memory or leak evidence.

| Variant | Minimum sample peak MiB | Maximum sample peak MiB |
| --- | ---: | ---: |
| Baseline VM | 15.273 | 94.066 |
| Baseline JIT | 15.527 | 31.332 |
| Latest VM | 15.391 | 92.438 |
| Latest JIT | 18.859 | 171.625 |
| Rust | 1.168 | 2.688 |
| Goose | 0.934 | 2.273 |

Host telemetry: 32 snapshots; CPU speed-limit reports 46–100%; swap occupancy 1205.0–1205.0 MiB; one-minute load 1.93–5.19. Speed-limit telemetry is not a frequency measurement or a valid timing correction factor. Swap occupancy alone does not prove an individual benchmark was swapping.

## Interpretation and limits

- Hardware: Intel Core i7-9750H, six physical/twelve logical CPUs, 16 GiB RAM, macOS 26.6.2, x86-64. Continuous load, memory, swap and available thermal telemetry were captured. No builds or profilers overlapped the samples.
- Same-machine execution reduces hardware differences. It cannot eliminate time-varying background work, thermal throttling, swap pressure or differences in how each runtime responds to those conditions. Alternating order and repeated rounds help; they do not make this a clean-host experiment.
- Small workload timings can be dominated by reporting resolution and startup. Read kernel and process-wall results separately. Large repeated gaps cannot reasonably be dismissed solely because this laptop was busy; a clean host is still required to quantify stable speed ratios.
- These test-size passes do not establish that all production-size workloads finish within practical time or memory limits. Historical production timeouts and the dedicated-host gate remain open.
- Fresh complete local build/release checks remain blocked by disk space. Existing identical-source Linux CI is reused evidence, not a new run. New-version artifacts and installation checks have not been performed.

## Evidence

The compressed `checkpoint-comparison-2026-11-02.json.gz` archive contains frozen identities, independently verified results, raw sample outputs, host telemetry, check logs and the verification/report scripts. The local working evidence is `.audit-evidence/checkpoint-2026-11-02/`.
