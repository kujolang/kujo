require 'json'
require 'digest'
root=File.expand_path(ARGV.fetch(0))
stages=%w[micro ownership ownership_controls phase neural neural_resources break_even break_even_confirmation].to_h{|s|[s,JSON.parse(File.read(root+'/'+s+'.json'))]}
expected={'micro'=>1,'ownership'=>225,'ownership_controls'=>270,'phase'=>18,'neural'=>28,'neural_resources'=>4,'break_even'=>24,'break_even_confirmation'=>32}
stages.each do |stage,report|
 abort "Wrong count #{stage}" unless report['rows'].size==expected[stage]
 report['hashes'].each{|p,h|abort "Changed binary/input #{p}" unless Digest::SHA256.file(p).hexdigest==h}
 report['rows'].each do |r|
  abort "Incomplete #{r['label']}" unless r['status']=='passed' && r['exitcode']==0
  if stage.start_with?('ownership')
   abort 'Ownership assertions missing' unless r['stdout'].lines.map(&:strip).include?("ISOLATION PASS #{r['shape']} #{r['owner']} #{r['size']}")
  end
  if stage=='phase'
   cfg=JSON.parse(File.read('benchmarks/langarena/test.json')).find{|c|c['name']==r['name']}
   abort 'Phase checksum missing' unless r['stdout'].lines.map(&:strip).include?(r['name']+': OK '+cfg['checksum'].to_s) && r['stdout'].lines.map(&:strip).include?('Summary: 1, 1, 0')
  end
  if stage=='neural_resources'
   abort 'Resource checksum missing' unless r['stdout'].lines.map(&:strip).include?('Etc::NeuralNet: OK 1586674334') && r['stdout'].lines.map(&:strip).include?('Summary: 1, 1, 0')
   rss=r['stderr'][/(\d+)\s+maximum resident set size/,1]
   abort 'Missing peak RSS' unless rss && rss.to_i>0
   r['peak_rss_bytes']=rss.to_i
  end
  if stage.start_with?('break_even')
   cfg=r['config'];n=r['iterations']
   wanted="encode aaaa... to YWFh...: #{40*(n+1)}".bytes.reduce(5381){|h,b|(h*33+b)&0xffffffff}
   abort 'Diagnostic config mismatch' unless cfg=={'name'=>'Base64::Encode','size'=>30,'iterations'=>n,'warmup_iterations'=>1,'checksum'=>wanted}
   abort 'Break-even checksum missing' unless r['stdout'].lines.map(&:strip).include?("Base64::Encode: OK #{wanted}") && r['stdout'].lines.map(&:strip).include?('Summary: 1, 1, 0')
  end
  if r['source_sha256']
   abort 'Source drift' unless Digest::SHA256.file(r['path']).hexdigest==r['source_sha256']
  end
  if %w[phase ownership ownership_controls].include?(stage) && r['runtime']!='interpreter'
   phases=r['stderr'].scan(/PHASE (\d+) requests=(\d+) bytes=(\d+)/).to_h{|p,n,b|[p,{'requests'=>n.to_i,'bytes'=>b.to_i}]}
   total=r['stderr'].match(/ALLOCATION_AUDIT .* enabled=true requests=(\d+) requested_bytes=(\d+)/)
   abort 'Invalid phase accounting' unless phases.size==6 && total && phases.values.sum{|v|v['requests']}==total[1].to_i && phases.values.sum{|v|v['bytes']}==total[2].to_i
   r['phases']=phases
   counters=r['stderr'].lines.find{|l|l.start_with?('COUNTERS ')}
   abort 'Missing runtime counters' unless counters
   r['measurements']=JSON.parse(counters.delete_prefix('COUNTERS '))
   abort 'Counter failure' unless r['measurements']['outcome']=='success'
  end
 end
end
neural=stages['neural']['rows']
neural.each do |r|
 abort 'Neural checksum missing' unless r['stdout'].lines.map(&:strip).include?('Etc::NeuralNet: OK 1586674334') && r['stdout'].lines.map(&:strip).include?('Summary: 1, 1, 0')
 warm=r['stdout'][/@profile warmup-done ns=(\d+)/,1];runs=r['stdout'].scan(/@profile run-\d+ ns=(\d+)/).flatten
 abort 'Missing timing checkpoints' unless warm && !runs.empty?
 r['loop_seconds']=(runs.last.to_i-warm.to_i)/1e9
 abort 'Nonpositive time' unless r['loop_seconds']>0
end
median=->(a){s=a.sort;s[s.size/2]}
stats=%w[vm jit].to_h do |mode|
 pairs=(1..7).map do |round|
  a=neural.find{|r|r['runtime']==mode && r['round']==round && r['variant']=='original'}
  b=neural.find{|r|r['runtime']==mode && r['round']==round && r['variant']=='candidate'}
  {round:round,loop_ratio:b['loop_seconds']/a['loop_seconds'],wall_ratio:b['elapsed_seconds']/a['elapsed_seconds']}
 end
 [mode,{pairs:pairs,median_loop_ratio:median.call(pairs.map{|p|p[:loop_ratio]}),median_wall_ratio:median.call(pairs.map{|p|p[:wall_ratio]})}]
end
manifest=JSON.parse(File.read(root+'/manifest.json'))
manifest['hashes'].each{|p,h|abort "Manifest drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
micro=stages['micro']['rows'][0]['stdout']
abort 'Missing direct controls' unless micro.lines.count{|l|l.start_with?('MICRO ')}==21 && micro.lines.count{|l|l.start_with?('arity=')}==6
prior=JSON.parse(File.read('benchmarks/langarena/priority-diagnosis-2026-10-25.json'))
sample_categories=prior['rows'].map do |r|
 profile=r['observations']['sample']['cpu_profile'].to_s
 block=profile.split('Sort by top of stack, same collapsed (when >= 5):')[1].to_s.split('Binary Images:')[0]
 counts=Hash.new(0)
 block.lines.each do |line|
  m=line.match(/^\s+(.+?)\s+(\d+)\s*$/);next unless m
  name=m[1]
  category=case name
  when /cranelift|regalloc/ then 'compiler'
  when /execute_dispatch|OpCode.*clone/ then 'vm_dispatch_or_opcode_clone'
  when /NativeCallRoots|NativeOwnedValues|jit_|kujo::jit::/ then 'jit_helpers_and_boundaries'
  when /findenv|getenv/ then 'environment_lookup'
  when /malloc|free|memmove|memcpy/ then 'allocator_or_memory_move'
  else 'other_or_unattributed'
  end
  counts[category]+=m[2].to_i
 end
 {name:r['name'],runtime:r['runtime'],top_stack_counts:counts,limitation:'Collapsed leaf counts omit symbols below five samples; no native/fallback time percentage, no kernel-only attribution.'}
end
result={scope:'Local isolation; unchanged runtime; clean-host comparisons pending',manifest:manifest,neural_stats:stats,sample_categories:sample_categories,stages:stages}
File.write('benchmarks/langarena/local-isolation-2026-10-25.json',JSON.pretty_generate(result)+"\n")
puts JSON.pretty_generate(stats)
puts 'Verified 495 ownership/parity observations, 18 phase observations, 28 NeuralNet comparisons, 56 break-even observations, four resource observations and direct controls.'
