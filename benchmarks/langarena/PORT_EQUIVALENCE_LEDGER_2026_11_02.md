# All-50 port-equivalence evidence ledger

This ledger enumerates exactly the 50 names in unchanged `test.json`, checked
against their source registrations. It tracks **structural equivalence**, not
checksum coverage, runtime optimization acceptance or a performance ranking.
Existing checkpoint checksum passes remain separate evidence.

Every row remains open. Bounded Rust oracles do not establish Goose equivalence,
production equivalence or equal operation counts. “Oracle open” means no dedicated
full-state oracle is indexed by this campaign ledger; it does not erase earlier
checksum tests or representative source research.

For each row, completion requires pinned comparator/source identities, equivalent
inputs and phase boundaries, complete-output/state checks on official and
adversarial controls, explicit work/layout differences, all-three-runtime Kujo
checks, and recorded production/Goose outcomes. Count differences must be
separated from runtime costs; diagnostic port variants cannot silently replace
the accepted port. No rows may be checked off based only on matching checksums.

| Workload | Kujo source | Current structural evidence | Next comparison gate |
| --- | --- | --- | --- |
| Binarytrees::Obj | [10_basic.kujo](src/10_basic.kujo) | [16 complete Rust/all-three-Kujo tree controls](TREE_PORT_STATE_AUDIT_2026_11_03.md); partial | Construction/traversal/copy counts, numeric boundaries, production and executed Goose state. |
| Binarytrees::Arena | [10_basic.kujo](src/10_basic.kujo) | [16 complete Rust/all-three-Kujo tree controls](TREE_PORT_STATE_AUDIT_2026_11_03.md); partial | Construction/traversal/copy counts, numeric boundaries, production and executed Goose state. |
| Brainfuck::Array | [20_text_hash.kujo](src/20_text_hash.kujo) | [Canonical bracket-jump fix:96 full state/count controls and150 checksum combinations pass](BF_ARRAY_JUMP_FIX_2026_11_03.md); partial | Production sequence running; executed comparator/output traces and invalid-input/empty/growth boundaries remain open. |
| Brainfuck::Recursion | [20_text_hash.kujo](src/20_text_hash.kujo) | [One-cell initialization fixed; 96 tape controls and 150 checksum combinations pass](BF_RECURSION_TAPE_FIX_2026_11_03.md); partial | Actual comparator execution, output/operation traces, large tape growth, malformed inputs, accumulator policy and production. |
| Matmul::Single | [25_matmul.kujo](src/25_matmul.kujo) | [1,152 complete matrix observations, 116,736 exactly checked cells](MATMUL_PORT_STATE_AUDIT_2026_11_03.md); partial | Canonical generation/general float policy, worker setup/transfer counts, executed Rust/Goose and production coverage. |
| Matmul::T4 | [25_matmul.kujo](src/25_matmul.kujo) | [1,152 complete matrix observations, 116,736 exactly checked cells](MATMUL_PORT_STATE_AUDIT_2026_11_03.md); partial | Canonical generation/general float policy, worker setup/transfer counts, executed Rust/Goose and production coverage. |
| Matmul::T8 | [25_matmul.kujo](src/25_matmul.kujo) | [1,152 complete matrix observations, 116,736 exactly checked cells](MATMUL_PORT_STATE_AUDIT_2026_11_03.md); partial | Canonical generation/general float policy, worker setup/transfer counts, executed Rust/Goose and production coverage. |
| Matmul::T16 | [25_matmul.kujo](src/25_matmul.kujo) | [1,152 complete matrix observations, 116,736 exactly checked cells](MATMUL_PORT_STATE_AUDIT_2026_11_03.md); partial | Canonical generation/general float policy, worker setup/transfer counts, executed Rust/Goose and production coverage. |
| Base64::Encode | [10_basic.kujo](src/10_basic.kujo) | [12-size full-data controls across all3Kujo modes](BASE64_PORT_STATE_AUDIT_2026_11_03.md); small-input encoding preview corrected with before/after regression; partial | Executed Rust/Goose state, byte-domain/malformed controls, work counts and production iterations. |
| Base64::Decode | [10_basic.kujo](src/10_basic.kujo) | [12-size decoded-data controls across all3Kujo modes](BASE64_PORT_STATE_AUDIT_2026_11_03.md); partial | Executed Rust/Goose state, short-input checksum boundary, byte-domain/malformed controls, work counts and production iterations. |
| Json::Generate | [10_basic.kujo](src/10_basic.kujo) | [360 JSON full-state observations across all three Kujo modes](JSON_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, serialization policy, malformed/empty controls, work counts, executed Rust/Goose and production coverage; DOM-then-mapping difference confirmed. |
| Json::ParseDom | [10_basic.kujo](src/10_basic.kujo) | [360 JSON full-state observations across all three Kujo modes](JSON_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, serialization policy, malformed/empty controls, work counts, executed Rust/Goose and production coverage; DOM-then-mapping difference confirmed. |
| Json::ParseMapping | [10_basic.kujo](src/10_basic.kujo) | [360 JSON full-state observations across all three Kujo modes](JSON_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, serialization policy, malformed/empty controls, work counts, executed Rust/Goose and production coverage; DOM-then-mapping difference confirmed. |
| CSV::Parse | [10_basic.kujo](src/10_basic.kujo) | [288 complete CSV field/point/checksum observations in all three Kujo modes](CSV_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, malformed-input policy, allocation/work counts, executed Rust/Goose and production coverage; full-field materialization documented. |
| Etc::Sieve | [10_basic.kujo](src/10_basic.kujo) | [216 complete flag-buffer observations in all three Kujo modes](SIEVE_PORT_STATE_AUDIT_2026_11_03.md); partial | Executed Rust/Goose controls, marking/traversal work counts, invalid limits and production coverage. |
| Etc::TextRaytracer | [46_ray.kujo](src/46_ray.kujo) | Oracle open; checksum/source evidence is separate | Every output pixel and ray/intersection work under the float policy. |
| Etc::NeuralNet | [49_neural.kujo](src/49_neural.kujo) | Oracle open; checksum/source evidence is separate | Weights and outputs after every training step; field/transfer work separately. |
| Etc::CacheSimulation | [47_cache.kujo](src/47_cache.kujo) | [36,000 complete request states and 36 checksums across all three Kujo runtimes](CACHE_PORT_STATE_AUDIT_2026_11_03.md); partial | Executed Rust/Goose state, broader/invalid capacities, work/allocation counts and production; string links versus node indices/references documented. |
| Etc::GameOfLife | [45_life.kujo](src/45_life.kujo) | [540 complete grid/link/checksum observations in all three Kujo modes](LIFE_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, invalid/zero dimensions, executed Rust/Goose, work/copy counts and production coverage. |
| Etc::Words | [20_text_hash.kujo](src/20_text_hash.kujo) | [288 complete frequency/order/winner/checksum observations](WORDS_PORT_STATE_AUDIT_2026_11_03.md); partial | Resolve comparator tie/tokenization boundaries, random preparation, work counts, executed Rust/Goose and production coverage. |
| Etc::LogParser | [48_logs.kujo](src/48_logs.kujo) | Oracle open; checksum/source evidence is separate | All parsed records/matches and malformed-line policy. |
| Template::Regex | [35_template.kujo](src/35_template.kujo) | Oracle open; checksum/source evidence is separate | Complete rendered output, escaping and parse/render work. |
| Template::Parse | [35_template.kujo](src/35_template.kujo) | Oracle open; checksum/source evidence is separate | Complete rendered output, escaping and parse/render work. |
| Sort::Quick | [30_sort.kujo](src/30_sort.kujo) | [1,764 complete wrapper observations on accepted runtime](SORT_WRAPPER_STATE_AUDIT_2026_11_03.md); partial | Preparation, empty/stability boundaries, comparison/call/copy counts, production and executed Rust/Goose controls. |
| Sort::Merge | [30_sort.kujo](src/30_sort.kujo) | [1,764 complete wrapper observations on accepted runtime](SORT_WRAPPER_STATE_AUDIT_2026_11_03.md); partial | Preparation, empty/stability boundaries, comparison/call/copy counts, production and executed Rust/Goose controls. |
| Sort::Self | [30_sort.kujo](src/30_sort.kujo) | [1,764 complete wrapper observations on accepted runtime](SORT_WRAPPER_STATE_AUDIT_2026_11_03.md); partial | Preparation, empty/stability boundaries, comparison/call/copy counts, production and executed Rust/Goose controls. |
| Graph::BFS | [40_graph.kujo](src/40_graph.kujo) | [Graph ordering controls](GRAPH_ORDER_AUDIT_2026_11_02.md); executed A* oracle, BFS/DFS source review | Full traversal/expansion traces on generated and adversarial graphs; production and Goose execution. |
| Graph::DFS | [40_graph.kujo](src/40_graph.kujo) | [Graph ordering controls](GRAPH_ORDER_AUDIT_2026_11_02.md); executed A* oracle, BFS/DFS source review | Full traversal/expansion traces on generated and adversarial graphs; production and Goose execution. |
| Graph::AStar | [40_graph.kujo](src/40_graph.kujo) | [Graph ordering controls](GRAPH_ORDER_AUDIT_2026_11_02.md); executed A* oracle, BFS/DFS source review | Full traversal/expansion traces on generated and adversarial graphs; production and Goose execution. |
| Hash::SHA256 | [20_text_hash.kujo](src/20_text_hash.kujo) | [2,052 full hash/digest/checksum observations across all three Kujo modes](HASH_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, operation/conversion/copy counts, production and executed Rust/Goose state; preserve toy SHA256 algorithm. |
| Hash::CRC32 | [20_text_hash.kujo](src/20_text_hash.kujo) | [2,052 full hash/digest/checksum observations across all three Kujo modes](HASH_PORT_STATE_AUDIT_2026_11_03.md); partial | Random preparation, operation/conversion/copy counts, production and executed Rust/Goose state; preserve toy SHA256 algorithm. |
| Calculator::Ast | [70_calculator.kujo](src/70_calculator.kujo) | Oracle open; checksum/source evidence is separate | Parsed structure, evaluated values, failures and token/operation work. |
| Calculator::Interpreter | [70_calculator.kujo](src/70_calculator.kujo) | Oracle open; checksum/source evidence is separate | Parsed structure, evaluated values, failures and token/operation work. |
| Maze::Generator | [80_maze.kujo](src/80_maze.kujo) | [40-state Rust/all-three-Kujo controls](MAZE_PORT_SOURCE_AUDIT_2026_11_02.md); partial | Expansion/finish-opening visit counts, disconnected/minimum-size controls, production and Goose execution. |
| Maze::BFS | [80_maze.kujo](src/80_maze.kujo) | [40-state Rust/all-three-Kujo controls](MAZE_PORT_SOURCE_AUDIT_2026_11_02.md); partial | Expansion/finish-opening visit counts, disconnected/minimum-size controls, production and Goose execution. |
| Maze::AStar | [80_maze.kujo](src/80_maze.kujo) | [40-state Rust/all-three-Kujo controls](MAZE_PORT_SOURCE_AUDIT_2026_11_02.md); partial | Expansion/finish-opening visit counts, disconnected/minimum-size controls, production and Goose execution. |
| CLBG::Fannkuchredux | [60_clbg.kujo](src/60_clbg.kujo) | Oracle open; checksum/source evidence is separate | Complete outputs/intermediate state under numeric policy and equivalent iteration work. |
| CLBG::Mandelbrot | [60_clbg.kujo](src/60_clbg.kujo) | Oracle open; checksum/source evidence is separate | Complete outputs/intermediate state under numeric policy and equivalent iteration work. |
| CLBG::Nbody | [61_nbody.kujo](src/61_nbody.kujo) | Oracle open; checksum/source evidence is separate | Complete outputs/intermediate state under numeric policy and equivalent iteration work. |
| CLBG::Spectralnorm | [60_clbg.kujo](src/60_clbg.kujo) | Oracle open; checksum/source evidence is separate | Complete outputs/intermediate state under numeric policy and equivalent iteration work. |
| Compress::BWTEncode | [50_bwt.kujo](src/50_bwt.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::BWTDecode | [50_bwt.kujo](src/50_bwt.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::HuffEncode | [51_huffman.kujo](src/51_huffman.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::HuffDecode | [51_huffman.kujo](src/51_huffman.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::ArithEncode | [52_arithmetic.kujo](src/52_arithmetic.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::ArithDecode | [52_arithmetic.kujo](src/52_arithmetic.kujo) | Oracle open; checksum/source evidence is separate | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::LZWEncode | [53_lzw.kujo](src/53_lzw.kujo) | [Eight full-byte Rust/all-three-Kujo controls](LZW_PORT_AUDIT_2026_11_02.md); partial | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Compress::LZWDecode | [53_lzw.kujo](src/53_lzw.kujo) | [Eight full-byte Rust/all-three-Kujo controls](LZW_PORT_AUDIT_2026_11_02.md); partial | Complete encoded bytes, decoded roundtrips and dictionary/tree/model transitions. |
| Distance::Jaro | [20_text_hash.kujo](src/20_text_hash.kujo) | [400 pairs/4,800 Kujo scores](DISTANCE_PAIR_AUDIT_2026_11_03.md), [192 states against Rust](DISTANCE_STATE_AUDIT_2026_11_03.md), [800 Goose scores](DISTANCE_GOOSE_AUDIT_2026_11_03.md), [64 Goose states](DISTANCE_GOOSE_STATE_2026_11_03.md); partial | Wider config/numeric boundaries, conversion/work counts and production; per-call byte encoding differs. |
| Distance::NGram | [20_text_hash.kujo](src/20_text_hash.kujo) | [400 pairs/4,800 Kujo scores plus multiset oracle](DISTANCE_PAIR_AUDIT_2026_11_03.md), [192 states against Rust](DISTANCE_STATE_AUDIT_2026_11_03.md), [800 Goose scores](DISTANCE_GOOSE_AUDIT_2026_11_03.md), [64 Goose states](DISTANCE_GOOSE_STATE_2026_11_03.md); partial | Wider config/numeric boundaries, conversion/work counts and production; preserve distinct-key denominator and short-input differences. |

The current implementation priority is completion of the separate memory-lifetime
gates before ownership-safe code/worker reuse; see the active campaign ledger. Independent port audits may proceed during
build waits, but cannot be counted as runtime speed improvements. Clean-host
measurements and all-50 production validation remain separate open gates.
