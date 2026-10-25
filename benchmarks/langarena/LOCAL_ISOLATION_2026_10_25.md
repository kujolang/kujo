# Local performance isolation — 2026-10-25

This completes the locally available measurement and isolation work for checklist
items 1 and 2. Runtime source is unchanged. Clean-host acceptance and precise
generated-native/fallback CPU-time attribution remain pending; this laptop lacks
xctrace and DTrace cannot initialize with current privileges/SIP.

## Local conclusions

- NeuralNet VM pairs show mixed directions, with no consistent slowdown. Raw median loop times are 13.1123 s original / 13.1460 s candidate; median paired ratio 0.908. Do not present that paired ratio as a controlled speedup. JIT is faster in 7/7 local pairs, median ratio 0.639. Clean-host confirmation remains pending.
- Simple global indexed assignment has a reproduced copy/whole-container validation cost. Function-local plain arrays/dictionaries/helpers do not show the same width-dependent repeated-copy pattern. The actual benchmark state is function-local, so do not attribute NeuralNet's timing to the global-only finding.
- Native call controls do not reproduce per-call allocation growth with arity. Named opcode cloning, debug environment lookup, path metadata, roots and kernel allocation traffic remain measured optimization targets.
- NeuralNet VM training-loop allocation traffic is about 1.66 GB for this test input; JIT is about 0.95 GB. These are cumulative requested bytes, not retained memory or peak RSS.
- No runtime fix, benchmark rewrite, production completion or competitive ranking is claimed.

## NeuralNet regression check

Seven paired rounds per runtime use original `f037705` and candidate `ea0ea3e`,
the same unchanged port/configuration, and alternating revision order. All 28
checksums passed. Ratios below are candidate/original (above 1 means slower).
Loop time is the final run checkpoint minus warmup completion; it includes the
small reset/checkpoint boundary cost. Whole-process time is recorded separately.

| Mode | Original median loop s | Candidate median loop s | Median paired loop ratio | Paired ratio range | Median paired wall ratio |
| --- | ---: | ---: | ---: | --- | ---: |
| vm | 13.1123 | 13.1460 | 0.908 | 0.760–1.063 | 0.919 |
| jit | 12.4806 | 8.1941 | 0.639 | 0.545–0.840 | 0.673 |

This is a local paired regression check, not a clean-machine ranking or proof of which commit caused a difference. Inspect individual paired results and thermal/load telemetry in the JSON before attributing a ratio to code.

## Separate NeuralNet resource observations

One external resource-accounting observation per variant, excluded from paired timing statistics. Peak RSS is whole-process memory, not cumulative allocation traffic.

| Mode | Revision | Peak RSS bytes |
| --- | --- | ---: |
| vm | original | 19079168 |
| vm | candidate | 18546688 |
| jit | original | 19333120 |
| jit | candidate | 35708928 |

## Ownership and scaling

All 225 observations passed: five shapes × three ownership states × five size/operation settings × VM/interpreter/JIT. The interpreter checks semantics; allocation accounting applies to VM/JIT. The following numbers are loop-phase requested bytes, including assertions and instrumentation, not peak heap.

| Shape | Mode | Owner | n=128, ops=64 | n=512, ops=64 | n=2048, ops=64 | n=2048, ops=1024 |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| array | vm | unique | 1644258 | 5773026 | 22288098 | 356528418 |
| array | vm | once | 1899106 | 6027874 | 22542946 | 360605986 |
| array | vm | each | 1899682 | 6028450 | 22543522 | 360615202 |
| array | jit | unique | 1644258 | 5773026 | 22288098 | 356528418 |
| array | jit | once | 1899106 | 6027874 | 22542946 | 360605986 |
| array | jit | each | 1899682 | 6028450 | 22543522 | 360615202 |
| nested | vm | unique | 1453806 | 4808430 | 18226926 | 290530926 |
| nested | vm | once | 1711406 | 5066030 | 18484526 | 294652526 |
| nested | vm | each | 1711982 | 5066606 | 18485102 | 294661742 |
| nested | jit | unique | 1453806 | 4808430 | 18226926 | 290530926 |
| nested | jit | once | 1711406 | 5066030 | 18484526 | 294652526 |
| nested | jit | each | 1711982 | 5066606 | 18485102 | 294661742 |
| dictionary | vm | unique | 3117906 | 11424594 | 44651346 | 714361746 |
| dictionary | vm | once | 3412946 | 11719634 | 44946386 | 719082386 |
| dictionary | vm | each | 3413522 | 11720210 | 44946962 | 719091602 |
| dictionary | jit | unique | 3117906 | 11424594 | 44651346 | 714361746 |
| dictionary | jit | once | 3412946 | 11719634 | 44946386 | 719082386 |
| dictionary | jit | each | 3413522 | 11720210 | 44946962 | 719091602 |
| helper | vm | unique | 1463266 | 4805602 | 18174946 | 290697826 |
| helper | vm | once | 1718114 | 5060450 | 18429794 | 294775394 |
| helper | vm | each | 1718690 | 5061026 | 18430370 | 294784610 |
| helper | jit | unique | 1463350 | 4805686 | 18175030 | 293419814 |
| helper | jit | once | 1718198 | 5060534 | 18429878 | 297497758 |
| helper | jit | each | 1718774 | 5061110 | 18430454 | 297506598 |
| struct_rows | vm | unique | 1564102 | 5055462 | 19020806 | 300432034 |
| struct_rows | vm | once | 1822342 | 5313702 | 19279046 | 304563874 |
| struct_rows | vm | each | 1822918 | 5314278 | 19279622 | 304573090 |
| struct_rows | jit | unique | 1564102 | 5055462 | 19020806 | 300432034 |
| struct_rows | jit | once | 1822342 | 5313702 | 19279046 | 304563874 |
| struct_rows | jit | each | 1822918 | 5314278 | 19279622 | 304573090 |

`once` keeps one pre-mutation snapshot; `each` takes a fresh snapshot before every write. Repeated copies in `each` are required by observable snapshot semantics. A size slope in `unique` needs attribution to validation, call transfer or an extra owner; it must not automatically be called a redundant copy. Direct Arc controls distinguish the mechanisms without changing runtime code.

## Scope and assertion controls

The additional 270 observations all passed. The table below removes native assertion calls on successful iterations and contrasts top-level/global state with function-local state. `unique` means no explicit user snapshot, not proof of a single internal owner. Counts are loop-phase requested bytes.

| Shape | Mode | Owner | Global n=2048 ops=64 | Local n=128 ops=64 | Local n=2048 ops=64 | Local n=2048 ops=1024 |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| array | vm | unique | 22032290 | 2805 | 2805 | 7605 |
| array | vm | once | 22032674 | 20253 | 281373 | 286173 |
| array | vm | each | 22033250 | 1119797 | 17831477 | 285266357 |
| array | jit | unique | 22032290 | 1825 | 1825 | 1825 |
| array | jit | once | 22032674 | 19273 | 280393 | 280393 |
| array | jit | each | 22033250 | 1118817 | 17830497 | 285260577 |
| nested | vm | unique | 378746 | 36261 | 97701 | 497061 |
| nested | vm | once | 381882 | 56461 | 379021 | 819661 |
| nested | vm | each | 17974830 | 1198593 | 17971713 | 286491393 |
| nested | jit | unique | 378746 | 26961 | 88401 | 358161 |
| nested | jit | once | 381882 | 44409 | 366969 | 636729 |
| nested | jit | each | 17974830 | 1187845 | 17960965 | 286309765 |
| dictionary | vm | unique | 44395858 | 82581 | 82581 | 1285461 |
| dictionary | vm | once | 44435410 | 160981 | 748501 | 2538901 |
| dictionary | vm | each | 44435986 | 2632917 | 40234197 | 643711317 |
| dictionary | jit | unique | 44395858 | 11201 | 11201 | 153281 |
| dictionary | jit | once | 44435410 | 54401 | 641921 | 843521 |
| dictionary | jit | each | 44435986 | 2527641 | 40128921 | 642017241 |
| helper | vm | unique | 17919138 | 86773 | 86773 | 1351093 |
| helper | vm | once | 17919522 | 104221 | 365341 | 1629661 |
| helper | vm | each | 17920098 | 1203765 | 17915445 | 286609845 |
| helper | jit | unique | 17919222 | 388961 | 388961 | 2000936 |
| helper | jit | once | 17919606 | 406409 | 667529 | 2287624 |
| helper | jit | each | 17920182 | 1505953 | 18217633 | 287267984 |
| struct_rows | vm | unique | 656530 | 67133 | 320637 | 1263065 |
| struct_rows | vm | once | 660306 | 135141 | 657445 | 2350593 |
| struct_rows | vm | each | 18769350 | 1309529 | 18766233 | 296402741 |
| struct_rows | jit | unique | 656530 | 55849 | 309353 | 1092421 |
| struct_rows | jit | once | 660306 | 120721 | 643025 | 2129773 |
| struct_rows | jit | each | 18769350 | 1296413 | 18753117 | 296183225 |

The source corroborates the global-array/dictionary cost: `src/compiler.rs` lowers simple global indexed assignment through a loaded value, generic `IndexSet`, then `StoreGlobal`; `src/vm.rs::index_set_result` uses `Arc::make_mut` and checks the returned aggregate's nesting depth. A live global owner remains while the temporary is mutated. Function locals use `IndexSetInPlace` instead. This is a reproduced global-binding copy/validation gap, not evidence that function-local benchmark state needs the same fix.

Local arrays/dictionaries/helpers with no user snapshot are flat across container widths in these controls. A single snapshot adds bounded detachment; fresh snapshots per write require repeated copies. Nested/struct rows retain a size-dependent first-write/validation component and per-path overhead; the operation-count controls do not show the global plain-array full-copy pattern. Preserve validation and snapshot semantics in any later fix.

JIT-enabled global array/dictionary/nested/struct controls recorded zero native function entries; their local function counterparts recorded one compiled exercise entry. Helper controls additionally execute native helper calls. Do not count the top-level checks as native-loop coverage.

## Allocation phases

All 18 instrumented full-workload checksum observations passed. Counts exclude frontend parsing/compilation and include main-thread VM execution only. Phases use private environment markers; their transition overhead is charged to the previous phase. JIT compilation belongs to whichever phase triggers it. These are diagnostic counts, not timings.

| Workload | Mode | Init bytes | Create bytes | Prepare bytes | Warmup bytes | Loop bytes | Checksum/report bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Sort::Quick | vm | 1152651 | 165192 | 247225798 | 559020994 | 1117678217 | 224873 |
| Sort::Quick | jit | 1175980 | 165344 | 45586157 | 36368992 | 28635284 | 224873 |
| Sort::Merge | vm | 1152235 | 165194 | 247225798 | 1721260372 | 3442200091 | 224777 |
| Sort::Merge | jit | 1176076 | 165344 | 45583481 | 987593398 | 1904499053 | 224777 |
| Etc::NeuralNet | vm | 1151982 | 165192 | 394240 | 830963879 | 1661615527 | 393500 |
| Etc::NeuralNet | jit | 1175317 | 165346 | 33548277 | 561822748 | 948480204 | 12795516 |
| Etc::CacheSimulation | vm | 1151572 | 166180 | 173053 | 75994172 | 457069635 | 231531 |
| Etc::CacheSimulation | jit | 1175943 | 166330 | 173341 | 122823405 | 441541554 | 231531 |
| Etc::GameOfLife | vm | 1151663 | 1860498 | 1864827 | 2419370 | 6731559 | 1091013 |
| Etc::GameOfLife | jit | 1175512 | 26967242 | 6565404 | 19743432 | 3479924 | 9148679 |
| Brainfuck::Array | vm | 1150128 | 165320 | 173059 | 10215554 | 20146806 | 226915 |
| Brainfuck::Array | jit | 1173979 | 165474 | 173347 | 73561466 | 20371612 | 226915 |
| Brainfuck::Recursion | vm | 1150228 | 165320 | 173059 | 10644775 | 20998972 | 226835 |
| Brainfuck::Recursion | jit | 1174087 | 165472 | 173347 | 54454528 | 20947306 | 226835 |
| Etc::TextRaytracer | vm | 1151378 | 165320 | 173053 | 498702437 | 1495646403 | 225669 |
| Etc::TextRaytracer | jit | 1175233 | 165472 | 173341 | 190649834 | 273580091 | 225669 |
| Base64::Encode | vm | 1150702 | 165192 | 178002 | 10997091 | 49721288 | 284759 |
| Base64::Encode | jit | 1174549 | 165344 | 178290 | 35949984 | 55384616 | 4540281 |

## Small-workload JIT payback

Base64 Encode size 30, one warmup, seven alternating VM/JIT pairs per iteration count. All 56 independently calculated diagnostic checksums passed. These changed iteration counts are not official benchmark passes. Ratios are median JIT whole-process time / median VM whole-process time.

| Iterations | VM median wall s | JIT median wall s | JIT/VM median wall ratio | Median paired ratio | Paired range |
| --- | ---: | ---: | ---: | ---: | --- |
| 32 | 0.1336 | 0.1545 | 1.156 | 1.179 | 0.893–1.406 |
| 512 | 0.1534 | 0.2930 | 1.910 | 2.074 | 1.413–2.884 |
| 8192 | 0.4263 | 0.7167 | 1.681 | 1.432 | 1.124–2.661 |
| 131072 | 6.1730 | 5.4469 | 0.882 | 0.923 | 0.609–1.234 |

The largest diagnostic case favors JIT at the median, but paired ratios cross 1.0. This does not establish a reliable admission threshold or universal break-even point on the loaded laptop. The smaller cases expose real compilation/transition costs; confirm policy choices on a clean host.

## Layout and call controls

Both revisions have 72-byte `OpCode`, 136-byte `Value` and 8-byte `BytecodeChunk` handles on this target. Layout growth therefore does not explain a revision difference. General-array snapshot copies still move 136-byte values: this is a structural cost shared by both revisions, not a new regression. The warmed native call control has no allocation for one call and only three small growth allocations across 128 calls at arities 1/4/8. It does not reproduce an arity-proportional allocation problem in that path.

## Direct mechanism controls

Setup/compilation/warmup are outside the counted call region. Allocation counts and requested bytes are structural observations. The tiny control timings are local only; a cached boolean is a counterfactual control, not an approved change to dynamic environment semantics.

```text
MICRO mode=scalar_opcode size=128 operations=128 requests=0 bytes=0 elapsed_ns=16786
MICRO mode=named_opcode size=128 operations=128 requests=128 bytes=2304 elapsed_ns=70769
MICRO mode=unique_array size=128 operations=128 requests=0 bytes=0 elapsed_ns=12457
MICRO mode=one_snapshot size=128 operations=128 requests=2 bytes=17448 elapsed_ns=29250
MICRO mode=snapshot_each size=128 operations=128 requests=256 bytes=2233344 elapsed_ns=683175
MICRO mode=environment size=128 operations=1000000 requests=0 bytes=0 elapsed_ns=178438584
MICRO mode=cached_boolean_control size=128 operations=1000000 requests=0 bytes=0 elapsed_ns=3759341
MICRO mode=scalar_opcode size=512 operations=128 requests=0 bytes=0 elapsed_ns=5176
MICRO mode=named_opcode size=512 operations=128 requests=128 bytes=2304 elapsed_ns=39184
MICRO mode=unique_array size=512 operations=128 requests=0 bytes=0 elapsed_ns=4709
MICRO mode=one_snapshot size=512 operations=128 requests=2 bytes=69672 elapsed_ns=60481
MICRO mode=snapshot_each size=512 operations=128 requests=256 bytes=8918016 elapsed_ns=1875930
MICRO mode=environment size=512 operations=1000000 requests=0 bytes=0 elapsed_ns=167280072
MICRO mode=cached_boolean_control size=512 operations=1000000 requests=0 bytes=0 elapsed_ns=3879248
MICRO mode=scalar_opcode size=2048 operations=128 requests=0 bytes=0 elapsed_ns=7162
MICRO mode=named_opcode size=2048 operations=128 requests=128 bytes=2304 elapsed_ns=40029
MICRO mode=unique_array size=2048 operations=128 requests=0 bytes=0 elapsed_ns=4656
MICRO mode=one_snapshot size=2048 operations=128 requests=2 bytes=278568 elapsed_ns=361836
MICRO mode=snapshot_each size=2048 operations=128 requests=256 bytes=35656704 elapsed_ns=6392780
MICRO mode=environment size=2048 operations=1000000 requests=0 bytes=0 elapsed_ns=169720892
MICRO mode=cached_boolean_control size=2048 operations=1000000 requests=0 bytes=0 elapsed_ns=3762677
arity=1 calls=1 allocations=0 bytes=0
arity=1 calls=128 allocations=3 bytes=1344
arity=4 calls=1 allocations=0 bytes=0
arity=4 calls=128 allocations=3 bytes=1344
arity=8 calls=1 allocations=0 bytes=0
arity=8 calls=128 allocations=3 bytes=1344
```

## Verification and limits

The strict audit requires exact checksums, successful exits, preserved source/binary hashes, and phase totals equal to total allocations/bytes. Raw results, host telemetry, compiler/build provenance, opcode/value layouts and sampled stack categories are in `local-isolation-2026-10-25.json`. Reproduction is in `probes/LOCAL_ISOLATION.md`.

Clean-host comparisons stay pending. Short CPU samples include setup/compilation and omit low-frequency symbols from collapsed summaries; helper counts and native entries are not native CPU-time percentages. No performance fix, production pass, upstream submission or universal speedup is claimed.

The separate verification receipt is `local-isolation-validation-2026-10-25.json`. It records formatting/syntax checks and an audit rejection test with a deliberately corrupted NeuralNet checksum.
