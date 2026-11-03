# Local LangArena rerun — 2026-11-03

Accepted runtime and main port; four variants, all50 unchanged test-size workloads; unaccepted owner-reuse and candidate port changes excluded; no production or clean-host claim.

Seven rotating rounds, four variants, 50 unchanged test-size workloads; 1,400 passing samples. Binary/input hashes, exact sample order, raw checksums/timings/RSS and timing-only port instrumentation independently verified.

| Variant | Median suite kernel s | Median suite process s | Process range s |
| --- | ---: | ---: | --- |
| Kujo VM | 45.543530 | 76.165441 | 63.000–78.859 |
| Kujo JIT | 22.136709 | 48.858108 | 42.552–51.077 |
| Rust | 0.055000 | 1.123741 | 1.068–1.200 |
| Goose | 0.045000 | 1.139208 | 1.037–1.193 |

Per-benchmark medians below. Competitor kernel values reported as zero are below runner reporting resolution; no per-case ratio should be inferred from them. Laptop background load/thermals are uncontrolled. Process time includes startup, warmup and output. These are not production or clean-host scores.

| Benchmark | Kujo VM kernel s | Kujo JIT kernel s | Rust kernel s | Goose kernel s |
| --- | ---: | ---: | ---: | ---: |
| Binarytrees::Obj | 0.001107 | 0.028767 | 0.000000 | 0.000000 |
| Binarytrees::Arena | 0.001496 | 0.053358 | 0.000000 | 0.000000 |
| Brainfuck::Array | 0.104029 | 0.059917 | 0.000000 | 0.000000 |
| Brainfuck::Recursion | 0.007865 | 0.008638 | 0.000000 | 0.000000 |
| Matmul::Single | 0.314917 | 0.081716 | 0.000000 | 0.000000 |
| Matmul::T4 | 0.127603 | 0.342123 | 0.001000 | 0.000000 |
| Matmul::T8 | 0.115479 | 0.493885 | 0.001000 | 0.000000 |
| Matmul::T16 | 0.146652 | 0.903157 | 0.002000 | 0.000000 |
| Base64::Encode | 0.145010 | 0.220138 | 0.001000 | 0.001000 |
| Base64::Decode | 0.142866 | 0.220576 | 0.001000 | 0.001000 |
| Json::Generate | 0.000173 | 0.000198 | 0.000000 | 0.000000 |
| Json::ParseDom | 0.001180 | 0.002191 | 0.000000 | 0.000000 |
| Json::ParseMapping | 0.001473 | 0.002269 | 0.000000 | 0.000000 |
| CSV::Parse | 0.001517 | 0.002405 | 0.000000 | 0.000000 |
| Etc::Sieve | 0.014660 | 0.004362 | 0.000000 | 0.000000 |
| Etc::TextRaytracer | 3.916481 | 3.047675 | 0.001000 | 0.001000 |
| Etc::NeuralNet | 6.882823 | 4.459504 | 0.002000 | 0.002000 |
| Etc::CacheSimulation | 0.681255 | 0.644445 | 0.001000 | 0.001000 |
| Etc::GameOfLife | 0.039225 | 0.022798 | 0.000000 | 0.000000 |
| Etc::Words | 0.008657 | 0.008128 | 0.000000 | 0.000000 |
| Etc::LogParser | 0.000341 | 0.000352 | 0.000000 | 0.000000 |
| Template::Regex | 0.001043 | 0.001074 | 0.000000 | 0.000000 |
| Template::Parse | 0.015147 | 0.014880 | 0.000000 | 0.000000 |
| Sort::Quick | 7.949152 | 1.841193 | 0.017000 | 0.019000 |
| Sort::Merge | 20.415147 | 6.704120 | 0.020000 | 0.014000 |
| Sort::Self | 0.120600 | 0.123385 | 0.005000 | 0.005000 |
| Graph::BFS | 0.003773 | 0.002541 | 0.000000 | 0.000000 |
| Graph::DFS | 0.007067 | 0.004860 | 0.000000 | 0.000000 |
| Graph::AStar | 0.004570 | 0.003212 | 0.000000 | 0.000000 |
| Hash::SHA256 | 0.047200 | 0.036926 | 0.000000 | 0.000000 |
| Hash::CRC32 | 0.147993 | 0.104666 | 0.000000 | 0.000000 |
| Calculator::Ast | 0.021643 | 0.019378 | 0.000000 | 0.000000 |
| Calculator::Interpreter | 0.137616 | 0.195042 | 0.001000 | 0.000000 |
| Maze::Generator | 1.040657 | 0.764858 | 0.000000 | 0.000000 |
| Maze::BFS | 0.232954 | 0.159102 | 0.001000 | 0.000000 |
| Maze::AStar | 0.521966 | 0.355230 | 0.001000 | 0.001000 |
| CLBG::Fannkuchredux | 0.016006 | 0.004474 | 0.000000 | 0.000000 |
| CLBG::Mandelbrot | 0.264848 | 0.014718 | 0.000000 | 0.000000 |
| CLBG::Nbody | 0.550807 | 0.278070 | 0.000000 | 0.000000 |
| CLBG::Spectralnorm | 0.509704 | 0.253179 | 0.000000 | 0.000000 |
| Compress::BWTEncode | 0.030861 | 0.152145 | 0.000000 | 0.000000 |
| Compress::BWTDecode | 0.006323 | 0.002626 | 0.000000 | 0.000000 |
| Compress::HuffEncode | 0.011691 | 0.055720 | 0.000000 | 0.000000 |
| Compress::HuffDecode | 0.005880 | 0.051811 | 0.000000 | 0.000000 |
| Compress::ArithEncode | 0.014233 | 0.008989 | 0.000000 | 0.000000 |
| Compress::ArithDecode | 0.021084 | 0.014207 | 0.000000 | 0.000000 |
| Compress::LZWEncode | 0.010850 | 0.009074 | 0.000000 | 0.000000 |
| Compress::LZWDecode | 0.004309 | 0.003893 | 0.000000 | 0.000000 |
| Distance::Jaro | 0.001905 | 0.003201 | 0.000000 | 0.000000 |
| Distance::NGram | 0.021278 | 0.018125 | 0.000000 | 0.000000 |

Accepted source: cd5f2b207a8f907d6c263f5a2e405412965344ca. Frozen Kujo SHA-256: 17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325. Comparator binaries are identical to the November 2 comparison. Unaccepted worker reuse and candidate calculator/log port corrections are excluded. Full identities, raw data and host telemetry are preserved in the companion archive.

## Process-time medians by benchmark

| Benchmark | Kujo VM s | Kujo JIT s | Rust s | Goose s |
| --- | ---: | ---: | ---: | ---: |
| Binarytrees::Obj | 0.072173 | 0.096829 | 0.017185 | 0.017529 |
| Binarytrees::Arena | 0.068191 | 0.121722 | 0.015892 | 0.014157 |
| Brainfuck::Array | 0.223987 | 0.333129 | 0.014853 | 0.014634 |
| Brainfuck::Recursion | 0.078638 | 0.180678 | 0.024728 | 0.015323 |
| Matmul::Single | 0.502963 | 0.271955 | 0.023997 | 0.023131 |
| Matmul::T4 | 0.251359 | 0.574642 | 0.014680 | 0.025921 |
| Matmul::T8 | 0.231419 | 0.762185 | 0.024098 | 0.024924 |
| Matmul::T16 | 0.271589 | 1.319750 | 0.025458 | 0.034739 |
| Base64::Encode | 0.261548 | 0.420979 | 0.025462 | 0.026046 |
| Base64::Decode | 0.241575 | 0.432605 | 0.025639 | 0.025132 |
| Json::Generate | 0.073332 | 0.283732 | 0.016827 | 0.025534 |
| Json::ParseDom | 0.075567 | 0.300779 | 0.024084 | 0.024439 |
| Json::ParseMapping | 0.079108 | 0.300179 | 0.023941 | 0.024775 |
| CSV::Parse | 0.080050 | 0.314248 | 0.025235 | 0.023332 |
| Etc::Sieve | 0.095944 | 0.171323 | 0.025579 | 0.025718 |
| Etc::TextRaytracer | 5.351792 | 4.397560 | 0.025001 | 0.024678 |
| Etc::NeuralNet | 10.605593 | 7.030520 | 0.025749 | 0.025967 |
| Etc::CacheSimulation | 0.878042 | 0.973482 | 0.025077 | 0.022979 |
| Etc::GameOfLife | 0.148643 | 0.253890 | 0.016724 | 0.016824 |
| Etc::Words | 0.110761 | 0.204170 | 0.024233 | 0.024406 |
| Etc::LogParser | 0.075335 | 0.257661 | 0.025481 | 0.016209 |
| Template::Regex | 0.078517 | 0.166064 | 0.015961 | 0.024861 |
| Template::Parse | 0.099241 | 0.286282 | 0.022818 | 0.015209 |
| Sort::Quick | 12.755643 | 3.587691 | 0.049850 | 0.046524 |
| Sort::Merge | 32.062604 | 10.798835 | 0.054264 | 0.043431 |
| Sort::Self | 1.152640 | 0.874332 | 0.027481 | 0.026899 |
| Graph::BFS | 0.072047 | 0.249906 | 0.024618 | 0.014060 |
| Graph::DFS | 0.079186 | 0.264137 | 0.016890 | 0.017576 |
| Graph::AStar | 0.072973 | 0.381317 | 0.018122 | 0.015171 |
| Hash::SHA256 | 0.137328 | 0.400072 | 0.021962 | 0.015977 |
| Hash::CRC32 | 0.272761 | 0.492285 | 0.017215 | 0.024870 |
| Calculator::Ast | 0.094605 | 0.543192 | 0.017002 | 0.025413 |
| Calculator::Interpreter | 0.246477 | 0.834952 | 0.022873 | 0.014988 |
| Maze::Generator | 2.068760 | 1.818905 | 0.025352 | 0.025096 |
| Maze::BFS | 1.297124 | 1.408753 | 0.025913 | 0.025990 |
| Maze::AStar | 1.767787 | 1.830855 | 0.026384 | 0.024695 |
| CLBG::Fannkuchredux | 0.088360 | 0.299069 | 0.022673 | 0.024307 |
| CLBG::Mandelbrot | 0.423063 | 0.380120 | 0.017165 | 0.022747 |
| CLBG::Nbody | 0.887870 | 0.903100 | 0.024572 | 0.024833 |
| CLBG::Spectralnorm | 0.676317 | 0.441190 | 0.016995 | 0.016861 |
| Compress::BWTEncode | 0.105482 | 0.366626 | 0.015864 | 0.024290 |
| Compress::BWTDecode | 0.084192 | 0.319036 | 0.014619 | 0.013935 |
| Compress::HuffEncode | 0.083330 | 0.222536 | 0.017496 | 0.016594 |
| Compress::HuffDecode | 0.078619 | 0.268177 | 0.023604 | 0.016993 |
| Compress::ArithEncode | 0.088694 | 0.311293 | 0.018071 | 0.023714 |
| Compress::ArithDecode | 0.101627 | 0.567953 | 0.015828 | 0.015550 |
| Compress::LZWEncode | 0.078661 | 0.172762 | 0.017146 | 0.024204 |
| Compress::LZWDecode | 0.080094 | 0.226589 | 0.024408 | 0.015450 |
| Distance::Jaro | 0.073599 | 0.379833 | 0.018260 | 0.015123 |
| Distance::NGram | 0.120481 | 0.267219 | 0.023866 | 0.025609 |

Host telemetry: 30 snapshots; one-minute load 1.32–6.43; CPU speed-limit reports 55–100%; swap occupied 1616.75 MiB. These do not justify a timing correction or identify which process experienced memory pressure.

JIT combined process-time ratio: 43.48× Rust, 42.89× Goose. These are ratios of median suite totals, not geometric-mean benchmark scores. Kujo JIT computation was 2.06× faster than Kujo VM.

No runtime edits, rebuilds, release/tag, or upstream contact occurred. The broader optimization campaign remains paused. Compared with the November 2 checkpoint, all four current process totals are lower; changing host conditions and accepted source changes prevent attributing that difference to runtime improvements.
