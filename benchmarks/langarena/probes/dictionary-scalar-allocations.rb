# Paired scalar dictionary allocation gate; retain integer and snapshot controls.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 4
baseline, candidate, binary, out = ARGV.map { |p| File.expand_path(p) }
[baseline, candidate, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[baseline,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[],sources:{}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 %w[int float bool null mixed].product([2,128,2048],%w[once fresh]).each do |kind,width,snapshot|
  dir=out+"/#{kind}-#{width}-#{snapshot}";FileUtils.mkdir_p(dir)
  expr={'int'=>'n','float'=>'n + 0.5','bool'=>'n % 2 == 0','null'=>'null'}[kind]
  body=expr ? "state[0] = #{expr}" : 'if n % 4 == 0 { state[0] = n } else if n % 4 == 1 { state[0] = n + 0.5 } else if n % 4 == 2 { state[0] = true } else { state[0] = null }'
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func edit(n) { #{body}\nreturn state[0] }\n"
  File.write(dir+'/owner.kujo',mod);r[:sources][Digest::SHA256.hexdigest(mod)]=mod
  [16,32].each do |calls|
   expected_body=expr ? "return #{expr}" : 'if n % 4 == 0 { return n } else if n % 4 == 1 { return n + 0.5 } else if n % 4 == 2 { return true } else { return null }'
   saved=snapshot=='fresh' ? 'let saved := snapshot() let old := saved[0]' : ''
   check=snapshot=='fresh' ? 'assert_equal(saved[0], old)' : ''
   source="from owner import edit, snapshot\nfunc expected(n) { #{expected_body} }\nlet original := snapshot()\nmut i := 1\nwhile i <= #{calls} { #{saved}\nassert_equal(edit(i), expected(i))\n#{check}\ni += 1 }\nassert_equal(original[0], 0)\nassert_equal(original[1], 1)\nassert_equal(snapshot()[0], expected(#{calls}))\nassert_equal(snapshot()[1], 1)\nprint(\"SCALAR DICTIONARY PASS\")\n"
   path=dir+"/calls-#{calls}.kujo";File.write(path,source);r[:sources][Digest::SHA256.hexdigest(source)]=source
   meta={kind:kind,width:width,snapshot:snapshot,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(source)}
   %w[vm jit].each do |mode|
    pair=[baseline,candidate].map do |probe|
     stdout,stderr,status=Open3.capture3(probe,mode,path);raise stdout+stderr unless status.success?&&stdout=="SCALAR DICTIONARY PASS\n"
     counts=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless counts
     {requests:counts[1].to_i,bytes:counts[2].to_i}
    end
    r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
   end
   %w[vm interpreter jit].each do |mode|
    args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
    stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="SCALAR DICTIONARY PASS\n"
    r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
   end
  end
 end
 raise 'Coverage' unless r[:rows].size==120&&r[:semantics].size==180
r[:rows].each do |row|
 raise "Total allocation regression #{row}" unless %i[requests bytes].all?{|m|row[:candidate][m]<=row[:baseline][m]}
end
r[:deltas]=r[:rows].group_by{|x|x.values_at(:kind,:width,:snapshot,:mode)}.map do |key,rows|
 a,b=rows.sort_by{|x|x[:calls]}
 delta={kind:key[0],width:key[1],snapshot:key[2],mode:key[3]}
 %i[baseline candidate].each{|label|delta[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/16.0]}}
 raise "Added-call regression #{key}" unless %i[requests bytes].all?{|m|delta[:candidate][m]<=delta[:baseline][m]}
 if key[0]!='int'&&key[1]>=128&&key[2]=='once'
  raise "Missing target improvement #{key}" unless delta[:candidate][:bytes]<delta[:baseline][:bytes]
 end
 delta
end
r[:deltas].group_by{|x|x.values_at(:kind,:snapshot,:mode)}.each do |key,rows|
 next unless key[1]=='once'
 pair=rows.select{|x|x[:width]>=128}.sort_by{|x|x[:width]}
 raise "Width scaling #{key}" unless pair.size==2&&pair[0][:candidate]==pair[1][:candidate]
end
r[:deltas].group_by{|x|x.values_at(:kind,:width,:snapshot)}.each do |key,rows|
 raise "VM/JIT disagreement #{key}" unless rows.size==2&&rows.map{|x|x.values_at(:baseline,:candidate)}.uniq.size==1
end
r[:status]='passed';save.call;puts 'Dictionary scalar gate passed:120 allocation pairs /180 semantics'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
