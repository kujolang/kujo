require 'json'
require 'digest'
root=File.expand_path(ARGV.fetch(0))
read=->(p){File.read(File.join(root,p))}
configs=JSON.parse(File.read('benchmarks/langarena/test.json'))
cases=%w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::CacheSimulation Etc::GameOfLife Brainfuck::Array Brainfuck::Recursion Etc::TextRaytracer Base64::Encode]
rows=[]
cases.each do |name|
 cfg=configs.find{|r|r['name']==name}; abort 'missing config' unless cfg
 %w[vm jit].each do |mode|
  base="#{mode}-#{name.gsub('::','-')}"
  row={name:name,runtime:mode,config:cfg,observations:{}}
  %w[count sample counters].each do |kind|
   stem=base+'-'+kind
   stdout=read.call(stem+'.stdout');stderr=read.call(stem+'.stderr');status=Integer(read.call(stem+'.exit').strip, 10)
   abort "Failed #{stem}" unless status==0 && stdout.lines.map(&:strip).include?(name+': OK '+cfg['checksum'].to_s) && stdout.lines.map(&:strip).include?('Summary: 1, 1, 0')
   observation={exitcode:status,stdout:stdout,stderr:stderr,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
   if kind=='count'
    m=stderr.match(/enabled=true requests=(\d+) requested_bytes=(\d+)/);abort 'missing allocator count' unless m
    observation[:allocations]=m[1].to_i;observation[:requested_bytes]=m[2].to_i
   elsif kind=='sample'
    abort 'Sample allocation counting enabled' unless stderr.include?('enabled=false requests=0 requested_bytes=0')
    observation[:sampler_log]=read.call(stem+'.sampler.log')
    if File.exist?(File.join(root,stem+'.cpu.txt'))
     cpu=read.call(stem+'.cpu.txt');observation[:cpu_profile]=cpu;observation[:cpu_sha256]=Digest::SHA256.hexdigest(cpu)
     observation[:top_stack]=cpu.split('Sort by top of stack, same collapsed (when >= 5):')[1].to_s.split('Binary Images:')[0].lines.first(21).map(&:strip).reject(&:empty?)
    else
     observation[:cpu_profile]=nil
    end
   else
    bytes=read.call(stem+'.json');abort 'Invalid measurement size' unless bytes.bytesize.between?(1,8192)
    m=JSON.parse(bytes);abort 'Invalid counter result' unless m['schema']=='kujo.runtime-measurements/v1' && m['outcome']=='success'
    rss=stderr[/(\d+)\s+maximum resident set size/,1];abort 'Missing RSS' unless rss && rss.to_i>0
    observation[:maximum_resident_bytes]=rss.to_i;observation[:measurements]=m
   end
   row[:observations][kind]=observation
  end
  rows << row
 end
end
sustained=[]
%w[Etc::CacheSimulation Etc::GameOfLife Brainfuck::Array Brainfuck::Recursion Base64::Encode].each do |name|
 cfg=configs.find{|r|r['name']==name}
 %w[vm jit].each do |mode|
  stem="#{mode}-#{name.gsub('::','-')}-sustained"
  repetitions=name=='Etc::CacheSimulation' ? 10 : 100
  out=read.call(stem+'.stdout');err=read.call(stem+'.stderr')
  abort "Incomplete sustained #{stem}" unless read.call(stem+'.exit').strip=='0' && read.call(stem+'.repetitions').strip.to_i==repetitions
  lines=out.lines.map(&:strip)
  abort 'Repeated checksum mismatch' unless lines.count(name+': OK '+cfg['checksum'].to_s)==repetitions && lines.count('Summary: 1, 1, 0')==repetitions
  expected=File.read('benchmarks/langarena/port.kujo').sub(/main\(\)\n\n\z/,"mut diagnostic_round := 0\nwhile diagnostic_round < #{repetitions} { main() diagnostic_round += 1 }\n")
  actual=read.call(stem+'-work/benchmarks/langarena/port.kujo')
  abort 'Changed diagnostic kernel' unless expected==actual
  abort 'Changed sustained config' unless read.call(stem+'-work/benchmarks/langarena/test.json')==File.read('benchmarks/langarena/test.json')
  cpu=File.exist?(root+'/'+stem+'.cpu.txt') ? read.call(stem+'.cpu.txt') : nil
  top=cpu.to_s.split('Sort by top of stack, same collapsed (when >= 5):')[1].to_s.split('Binary Images:')[0].lines.first(21).map(&:strip).reject(&:empty?)
  sustained << {name:name,runtime:mode,repetitions:repetitions,stdout:out,stderr:err,cpu_profile:cpu,top_stack:top,source_sha256:Digest::SHA256.hexdigest(actual)}
 end
end
manifest=JSON.parse(read.call('manifest.json'))
manifest['hashes'].each{|p,h|next if p.end_with?('priority-diagnostics.sh');abort "Changed input #{p}" unless Digest::SHA256.file(p).hexdigest==h}
manifest['final_orchestrator_sha256']=Digest::SHA256.file('benchmarks/langarena/probes/priority-diagnostics.sh').hexdigest
manifest['audit_sha256']=Digest::SHA256.file(__FILE__).hexdigest
manifest['sustained_orchestrator_sha256']=Digest::SHA256.file('benchmarks/langarena/probes/sustained-priority-diagnostics.sh').hexdigest
manifest['probe_sha256']=Digest::SHA256.file(root+'/priority-probe').hexdigest
result={scope:'Local test-size diagnostics, not production or controlled timing',manifest:manifest,rows:rows,sustained:sustained,host_during:read.call('host-during.log'),retained_cli_host:File.exist?(root+'/retained-cli-host.log') ? read.call('retained-cli-host.log') : nil,orchestrator_log:read.call('run.log'),limits:['No dedicated host. Host thermal limits and external load affect observations.','Main-thread allocation traffic includes VM setup of port globals, preparation, warmup, JIT compilation and checksum; excludes frontend parsing and bytecode compilation. Not retained heap.','Three-second sample windows may capture initialization or compilation, and short cases may have no usable profile.','Harness Rust LTO build differs from CLI binary; report observations separately, never compare their wall times as an optimization.','Initial probe lacked CLI null globals; those probe observations were rejected. CLI counter runs used the hash-verified ordinary binary and were retained. Corrected probe observations were rerun with matching CLI constant globals.','Original baseline and competitor executables were removed from temporary storage; no new competitor or baseline timing comparison.','Per-function native-time coverage and isolated ownership/scaling reproduction remain follow-up gates.']}
s=JSON.pretty_generate(result)+"\n";abort 'JSON roundtrip' unless JSON.parse(s)==JSON.parse(JSON.generate(result))
File.write('benchmarks/langarena/priority-diagnosis-2026-10-25.json',s)
puts "Verified #{rows.size} case/runtime combinations, #{rows.size*3} checksum-bearing observations."
rows.each{|r|o=r[:observations];puts "#{r[:name]} #{r[:runtime]} allocations=#{o['count'][:allocations]} bytes=#{o['count'][:requested_bytes]} native=#{o['counters'][:measurements]['counters']['jit_native_function_entries']}"}
