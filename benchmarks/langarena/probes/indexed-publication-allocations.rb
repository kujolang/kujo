# Paired allocation/scaling gate; elapsed time is deliberately not an acceptance metric.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR [array|dict|control]' unless [4,5].include?(ARGV.size)
baseline_probe,candidate_probe,binary,d=ARGV.first(4).map{|p|File.expand_path(p)}
target_shape=ARGV[4] || 'array'
raise 'Unsupported scaling target' unless %w[array dict control].include?(target_shape)
raise 'Output directory already exists' if File.exist?(d)
[baseline_probe,candidate_probe,binary].each{|p|raise 'Missing executable '+p unless File.executable?(p)}
FileUtils.mkdir_p(d)
fixtures=JSON.parse(File.read('benchmarks/langarena/module-owner-isolation-2026-10-26.json'))
report={status:'running',target_shape:target_shape,rows:[],semantics:[],probe_sha256:{baseline:Digest::SHA256.file(baseline_probe).hexdigest,candidate:Digest::SHA256.file(candidate_probe).hexdigest},binary_sha256:Digest::SHA256.file(binary).hexdigest,runner_sha256:Digest::SHA256.file(__FILE__).hexdigest}
save=->{File.write(d+'/module-paired.json',JSON.pretty_generate(report)+"\n")};save.call
begin
 fixtures['fixtures'].each do |f|
  dir=d+"/module-controls/#{f['shape']}-#{f['width']}";FileUtils.mkdir_p(dir)
  File.write(dir+'/owner_state.kujo',fixtures['modules'].fetch(f['module_sha256']))
  path=dir+"/#{f['snapshot']}-#{f['updates_per_call']}-#{f['calls']}.kujo";File.write(path,f['source'])
  raise 'Module identity mismatch' unless Digest::SHA256.file(dir+'/owner_state.kujo').hexdigest==f['module_sha256']
  raise 'Fixture identity mismatch' unless Digest::SHA256.file(path).hexdigest==f['source_sha256']
  metadata=f.reject{|k,v|k=='source'}
  %w[vm jit].each do |mode|
   row=metadata.merge(mode:mode)
   {'baseline'=>baseline_probe,'candidate'=>candidate_probe}.each do |label,probe|
    out,err,status=Open3.capture3(probe,mode,path)
    raise out+err unless status.success? && out=="MODULE OWNER ISOLATION PASS\n"
    counts=err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise err unless counts
    row[label]={requests:counts[1].to_i,bytes:counts[2].to_i}
   end
   report[:rows]<<row;save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path];args<<'--'+mode unless mode=='vm'
   out,err,status=Open3.capture3(*args);raise out+err unless status.success?&&out=="MODULE OWNER ISOLATION PASS\n"
   report[:semantics]<<metadata.merge(mode:mode,exit:status.exitstatus);save.call
  end
 end
 raise 'Missing cases' unless report[:rows].size==128 && report[:semantics].size==192
 report[:reductions]=report[:rows].count{|r|r['candidate'][:bytes]<r['baseline'][:bytes]}
 report[:regressions]=report[:rows].select{|r|r['candidate'][:bytes]>r['baseline'][:bytes] || r['candidate'][:requests]>r['baseline'][:requests]}
 report[:status]='measured';save.call
 raise 'Allocation regressions require review' unless report[:regressions].empty?
 # Control mode retains the complete no-regression/semantic matrix when the
 # targeted improvement has a separate gate (for example dictionary key reads).
 if target_shape != 'control'
 raise 'No actual runtime allocation reduction' unless report[:reductions]>0
report[:scaling]=%w[vm jit].product([1,8]).map do |mode,updates|
 slopes=[128,2048].map do |width|
  group=report[:rows].select{|r|r[:mode]==mode && r['shape']==target_shape && r['snapshot']=='once' && r['width']==width && r['updates_per_call']==updates}
  raise 'Missing target scaling rows' unless group.size==2
  first,last=group.sort_by{|r|r['calls']}
  {width:width,baseline_bytes:last['baseline'][:bytes]-first['baseline'][:bytes],candidate_bytes:last['candidate'][:bytes]-first['candidate'][:bytes],candidate_requests:last['candidate'][:requests]-first['candidate'][:requests]}
 end
 {mode:mode,updates:updates,slopes:slopes}
end
save.call
raise 'Baseline width-copy cost not reproduced' unless report[:scaling].all?{|r|r[:slopes][1][:baseline_bytes]>r[:slopes][0][:baseline_bytes]}
raise 'Candidate per-call width-copy cost remains' unless report[:scaling].all?{|r|r[:slopes][1][:candidate_bytes]==r[:slopes][0][:candidate_bytes] && r[:slopes][1][:candidate_requests]==r[:slopes][0][:candidate_requests]}
end
report[:status]='passed';save.call

rescue=>e;report[:status]='failed';report[:error]=e.message;save.call;raise
end
