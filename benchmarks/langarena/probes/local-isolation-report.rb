require 'json'
j=JSON.parse(File.read('benchmarks/langarena/local-isolation-2026-10-25.json'))
median=->(a){a.sort[a.size/2]}
s=<<~MD
# Local performance isolation — 2026-10-25

This completes the locally available measurement and isolation work for checklist
items 1 and 2. Runtime source is unchanged. Clean-host acceptance and precise
generated-native/fallback CPU-time attribution remain pending; this laptop lacks
xctrace and DTrace cannot initialize with current privileges/SIP.

## NeuralNet regression check

Seven paired rounds per runtime use original `f037705` and candidate `ea0ea3e`,
the same unchanged port/configuration, and alternating revision order. All 28
checksums passed. Ratios below are candidate/original (above 1 means slower).
Loop time is the final run checkpoint minus warmup completion; it includes the
small reset/checkpoint boundary cost. Whole-process time is recorded separately.

| Mode | Original median loop s | Candidate median loop s | Median paired loop ratio | Paired ratio range | Median paired wall ratio |
| --- | ---: | ---: | ---: | --- | ---: |
MD
%w[vm jit].each do |mode|
 rows=j['stages']['neural']['rows'].select{|r|r['runtime']==mode};stats=j['neural_stats'][mode];ratios=stats['pairs'].map{|p|p['loop_ratio']}
 s+="| #{mode} | #{format('%.4f',median.call(rows.select{|r|r['variant']=='original'}.map{|r|r['loop_seconds']}))} | #{format('%.4f',median.call(rows.select{|r|r['variant']=='candidate'}.map{|r|r['loop_seconds']}))} | #{format('%.3f',stats['median_loop_ratio'])} | #{format('%.3f–%.3f',ratios.min,ratios.max)} | #{format('%.3f',stats['median_wall_ratio'])} |\n"
end
s+="\nThis is a local paired regression check, not a clean-machine ranking or proof of which commit caused a difference. Inspect individual paired results and thermal/load telemetry in the JSON before attributing a ratio to code.\n\n## Ownership and scaling\n\nAll 225 observations passed: five shapes × three ownership states × five size/operation settings × VM/interpreter/JIT. The interpreter checks semantics; allocation accounting applies to VM/JIT. The following numbers are loop-phase requested bytes, including assertions and instrumentation, not peak heap.\n\n| Shape | Mode | Owner | n=128, ops=64 | n=512, ops=64 | n=2048, ops=64 | n=2048, ops=1024 |\n| --- | --- | --- | ---: | ---: | ---: | ---: |\n"
rows=j['stages']['ownership']['rows']
%w[array nested dictionary helper struct_rows].each do |shape|
 %w[vm jit].each do |mode|
  %w[unique once each].each do |owner|
   vals=[[128,64],[512,64],[2048,64],[2048,1024]].map{|n,ops|rows.find{|r|r['shape']==shape && r['runtime']==mode && r['owner']==owner && r['size']==n && r['operations']==ops}['phases']['4']['bytes']}
   s+="| #{shape} | #{mode} | #{owner} | #{vals.join(' | ')} |\n"
  end
 end
end
s+="\n`once` keeps one pre-mutation snapshot; `each` takes a fresh snapshot before every write. Repeated copies in `each` are required by observable snapshot semantics. A size slope in `unique` needs attribution to validation, call transfer or an extra owner; it must not automatically be called a redundant copy. Direct Arc controls distinguish the mechanisms without changing runtime code.\n\n## Allocation phases\n\nAll 18 instrumented full-workload checksum observations passed. Counts exclude frontend parsing/compilation and include main-thread VM execution only. Phases use private environment markers; their transition overhead is charged to the previous phase. JIT compilation belongs to whichever phase triggers it. These are diagnostic counts, not timings.\n\n| Workload | Mode | Init bytes | Create bytes | Prepare bytes | Warmup bytes | Loop bytes | Checksum/report bytes |\n| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |\n"
j['stages']['phase']['rows'].each{|r|s+="| #{r['name']} | #{r['runtime']} | #{(0..5).map{|p|r['phases'][p.to_s]['bytes']}.join(' | ')} |\n"}
s+="\n## Small-workload JIT payback\n\nBase64 Encode size 30, one warmup, seven alternating VM/JIT pairs per iteration count. All 56 independently calculated diagnostic checksums passed. These changed iteration counts are not official benchmark passes. Ratios are median JIT whole-process time / median VM whole-process time.\n\n| Iterations | VM median wall s | JIT median wall s | JIT/VM median wall ratio | Median paired ratio | Paired range |\n| --- | ---: | ---: | ---: | ---: | --- |\n"
[32,512,8192,131072].each do |n|
rows=(j['stages']['break_even']['rows']+j['stages']['break_even_confirmation']['rows']).select{|r|r['iterations']==n}
v=median.call(rows.select{|r|r['runtime']=='vm'}.map{|r|r['elapsed_seconds']})
k=median.call(rows.select{|r|r['runtime']=='jit'}.map{|r|r['elapsed_seconds']})
ratios=rows.group_by{|r|r['round']}.values.map{|pair|pair.find{|r|r['runtime']=='jit'}['elapsed_seconds']/pair.find{|r|r['runtime']=='vm'}['elapsed_seconds']}
s+="| #{n} | #{format('%.4f',v)} | #{format('%.4f',k)} | #{format('%.3f',k/v)} | #{format('%.3f',median.call(ratios))} | #{format('%.3f–%.3f',ratios.min,ratios.max)} |\n"
end
s+="\n## Direct mechanism controls\n\nSetup/compilation/warmup are outside the counted call region. Allocation counts and requested bytes are structural observations. The tiny control timings are local only; a cached boolean is a counterfactual control, not an approved change to dynamic environment semantics.\n\n```text\n"+j['stages']['micro']['rows'][0]['stdout']+"```\n\n## Verification and limits\n\nThe strict audit requires exact checksums, successful exits, preserved source/binary hashes, and phase totals equal to total allocations/bytes. Raw results, host telemetry, compiler/build provenance, opcode/value layouts and sampled stack categories are in `local-isolation-2026-10-25.json`. Reproduction is in `probes/LOCAL_ISOLATION.md`.\n\nClean-host comparisons stay pending. Short CPU samples include setup/compilation and omit low-frequency symbols from collapsed summaries; helper counts and native entries are not native CPU-time percentages. No performance fix, production pass, upstream submission or universal speedup is claimed.\n"
controls="## Scope and assertion controls\n\nThe additional 270 observations all passed. The table below removes native assertion calls on successful iterations and contrasts top-level/global state with function-local state. `unique` means no explicit user snapshot, not proof of a single internal owner. Counts are loop-phase requested bytes.\n\n| Shape | Mode | Owner | Global n=2048 ops=64 | Local n=128 ops=64 | Local n=2048 ops=64 | Local n=2048 ops=1024 |\n| --- | --- | --- | ---: | ---: | ---: | ---: |\n"
rows=j['stages']['ownership_controls']['rows']
%w[array nested dictionary helper struct_rows].each do |shape|
 %w[vm jit].each do |mode|
  %w[unique once each].each do |owner|
   vals=[['global_guarded',2048,64],['local_guarded',128,64],['local_guarded',2048,64],['local_guarded',2048,1024]].map{|scope,n,ops|rows.find{|r|r['shape']==shape && r['runtime']==mode && r['owner']==owner && r['scope']==scope && r['size']==n && r['operations']==ops}['phases']['4']['bytes']}
   controls+="| #{shape} | #{mode} | #{owner} | #{vals.join(' | ')} |\n"
  end
 end
end
controls+="\nThe source corroborates the global-array/dictionary cost: `src/compiler.rs` lowers simple global indexed assignment through a loaded value, generic `IndexSet`, then `StoreGlobal`; `src/vm.rs::index_set_result` uses `Arc::make_mut` and checks the returned aggregate's nesting depth. A live global owner remains while the temporary is mutated. Function locals use `IndexSetInPlace` instead. This is a reproduced global-binding copy/validation gap, not evidence that function-local benchmark state needs the same fix.\n\nLocal arrays/dictionaries/helpers with no user snapshot are flat across container widths in these controls. A single snapshot adds bounded detachment; fresh snapshots per write require repeated copies. Nested/struct rows retain a size-dependent first-write/validation component and per-path overhead; the operation-count controls do not show the global plain-array full-copy pattern. Preserve validation and snapshot semantics in any later fix.\n\nJIT-enabled global array/dictionary/nested/struct controls recorded zero native function entries; their local function counterparts recorded one compiled exercise entry. Helper controls additionally execute native helper calls. Do not count the top-level checks as native-loop coverage.\n\n"
s=s.sub('## Allocation phases',controls+'## Allocation phases')
resources="## Separate NeuralNet resource observations\n\nOne external resource-accounting observation per variant, excluded from paired timing statistics. Peak RSS is whole-process memory, not cumulative allocation traffic.\n\n| Mode | Revision | Peak RSS bytes |\n| --- | --- | ---: |\n"
j['stages']['neural_resources']['rows'].each{|r|resources+="| #{r['runtime']} | #{r['variant']} | #{r['peak_rss_bytes']} |\n"}
s=s.sub('## Ownership and scaling',resources+"\n## Ownership and scaling")
s=s.sub('## Direct mechanism controls',"## Layout and call controls\n\nBoth revisions have 72-byte `OpCode`, 136-byte `Value` and 8-byte `BytecodeChunk` handles on this target. Layout growth therefore does not explain a revision difference. General-array snapshot copies still move 136-byte values: this is a structural cost shared by both revisions, not a new regression. The warmed native call control has no allocation for one call and only three small growth allocations across 128 calls at arities 1/4/8. It does not reproduce an arity-proportional allocation problem in that path.\n\n## Direct mechanism controls")
summary="## Local conclusions\n\n- The suspected NeuralNet VM regression did not reproduce consistently: raw median loop times are essentially tied; paired ratios span faster and slower runs. The median paired ratio is 0.908, which must not be presented as a controlled 9% speedup. JIT is faster in all seven local pairs, median ratio 0.639; clean-host confirmation remains pending.\n- Simple global indexed assignment has a reproduced copy/whole-container validation cost. Function-local plain arrays/dictionaries/helpers do not show the same width-dependent repeated-copy pattern. The actual benchmark state is function-local, so do not attribute NeuralNet's timing to the global-only finding.\n- Native call controls do not reproduce per-call allocation growth with arity. Named opcode cloning, debug environment lookup, path metadata, roots and kernel allocation traffic remain measured optimization targets.\n- NeuralNet VM training-loop allocation traffic is about 1.66 GB for this test input; JIT is about 0.95 GB. These are cumulative requested bytes, not retained memory or peak RSS.\n- No runtime fix, benchmark rewrite, production completion or competitive ranking is claimed.\n\n"
vmratios=j['neural_stats']['vm']['pairs'].map{|p|p['loop_ratio']}
jitratios=j['neural_stats']['jit']['pairs'].map{|p|p['loop_ratio']}
vmrows=j['stages']['neural']['rows'].select{|r|r['runtime']=='vm'}
original_median=median.call(vmrows.select{|r|r['variant']=='original'}.map{|r|r['loop_seconds']})
candidate_median=median.call(vmrows.select{|r|r['variant']=='candidate'}.map{|r|r['loop_seconds']})
direction=if vmratios.min<1 && vmratios.max>1 then 'mixed directions, with no consistent slowdown' elsif vmratios.min>1 then 'a slower candidate in every pair' else 'no slower candidate pair' end
summary=summary.sub(/- The suspected NeuralNet VM regression.*?clean-host confirmation remains pending\./, "- NeuralNet VM pairs show #{direction}. Raw median loop times are #{format('%.4f',original_median)} s original / #{format('%.4f',candidate_median)} s candidate; median paired ratio #{format('%.3f',median.call(vmratios))}. Do not present that paired ratio as a controlled speedup. JIT is faster in #{jitratios.count{|r|r<1}}/7 local pairs, median ratio #{format('%.3f',median.call(jitratios))}. Clean-host confirmation remains pending.")
s=s.sub('## NeuralNet regression check',summary+'## NeuralNet regression check')
s=s.sub('## Layout and call controls',"The largest diagnostic case favors JIT at the median, but paired ratios cross 1.0. This does not establish a reliable admission threshold or universal break-even point on the loaded laptop. The smaller cases expose real compilation/transition costs; confirm policy choices on a clean host.\n\n## Layout and call controls")
s+="\nThe separate verification receipt is `local-isolation-validation-2026-10-25.json`. It records formatting/syntax checks and an audit rejection test with a deliberately corrupted NeuralNet checksum.\n"
File.write('benchmarks/langarena/LOCAL_ISOLATION_2026_10_25.md',s)

