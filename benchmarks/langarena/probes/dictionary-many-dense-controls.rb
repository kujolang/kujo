# Paired allocation gate for multiple existing dictionary keys and scalar transitions.
require 'json'; require 'digest'; require 'fileutils'; require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY NEW_OUTPUT_DIR [improvement|control]' unless [4,5].include?(ARGV.size)
policy=ARGV.size==5 ? ARGV.pop : 'improvement'
raise 'Invalid policy' unless %w[improvement control].include?(policy)
base,candidate,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(out)
[base,candidate,binary].each{|p|raise 'Missing executable' unless File.executable?(p)}
FileUtils.mkdir_p(out)
r={status:'running',policy:policy,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,rows:[],semantics:[],sources:{}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 [128,2048].product([128,2048],%w[once fresh],%w[int mixed]).reject{|width,writes,_,_|writes>width}.each do |width,writes,snapshot,kind|
  dir=out+"/#{width}-#{writes}-#{snapshot}-#{kind}";FileUtils.mkdir_p(dir)
  value=kind=='int' ? 'n + key' : 'if key % 4 == 0 { return n } else if key % 4 == 1 { return n + 0.5 } else if key % 4 == 2 { return n % 2 == 0 } else { return null }'
  update=kind=='int' ? 'state[key] = n + key' : 'if key % 4 == 0 { state[key] = n } else if key % 4 == 1 { state[key] = n + 0.5 } else if key % 4 == 2 { state[key] = n % 2 == 0 } else { state[key] = null }'
  expected=kind=='int' ? 'return n + key' : value
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func expected(n, key) { #{expected} }\nexport func edit(n) { mut key := 0 while key < #{writes} { #{update} key += 1 } return state[0] }\n"
  File.write(dir+'/owner.kujo',mod);r[:sources][Digest::SHA256.hexdigest(mod)]=mod
  [4,8].each do |calls|
   before=snapshot=='fresh' ? 'let saved := snapshot()' : ''
   after=snapshot=='fresh' ? 'mut key := 0 while key < WRITES { if n == 1 { assert_equal(saved[key], key) } else { assert_equal(saved[key], expected(n - 1, key)) } key += 1 }' : ''
   source="from owner import snapshot, expected, edit\nlet original := snapshot()\nmut n := 1\nwhile n <= #{calls} { #{before} assert_equal(edit(n), expected(n, 0)) #{after} n += 1 }\nlet result := snapshot()\nmut key := 0\nwhile key < #{width} { assert_equal(original[key], key) if key < WRITES { assert_equal(result[key], expected(#{calls}, key)) } else { assert_equal(result[key], key) } key += 1 }\nprint(\"MULTI KEY PASS\")\n".gsub('WRITES',writes.to_s)
   path=dir+"/calls-#{calls}.kujo";File.write(path,source);r[:sources][Digest::SHA256.hexdigest(source)]=source
   meta={width:width,writes:writes,snapshot:snapshot,kind:kind,calls:calls,source_sha256:Digest::SHA256.hexdigest(source),module_sha256:Digest::SHA256.hexdigest(mod)}
   %w[vm jit].each do |mode|
    pair=[base,candidate].map do |probe|
     stdout,stderr,status=Open3.capture3(probe,mode,path)
     raise stdout+stderr unless status.success?&&stdout=="MULTI KEY PASS\n"
     m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
     {requests:m[1].to_i,bytes:m[2].to_i}
    end
    r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
   end
   %w[vm interpreter jit].each do |mode|
    args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
    stdout,stderr,status=Open3.capture3(*args)
    raise stdout+stderr unless status.success?&&stdout=="MULTI KEY PASS\n"
    r[:semantics]<<meta.merge(mode:mode,exit:status.exitstatus);save.call
   end
  end
 end
 raise 'Coverage' unless r[:rows].size==48&&r[:semantics].size==72
 r[:rows].each{|x|raise "Total regression #{x}" unless %i[requests bytes].all?{|m|x[:candidate][m]<=x[:baseline][m]}}
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:width,:writes,:snapshot,:kind,:mode)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};d={width:key[0],writes:key[1],snapshot:key[2],kind:key[3],mode:key[4]}
  %i[baseline candidate].each{|label|d[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/4.0]}}
  raise "Added-call regression #{key}" unless %i[requests bytes].all?{|m|d[:candidate][m]<=d[:baseline][m]}
  if policy=='improvement'&&key[1]>1&&key[2]=='once'
   raise "Missing improvement #{key}" unless d[:candidate][:bytes]<d[:baseline][:bytes]
  end
  d
 end
 r[:deltas].group_by{|x|x.values_at(:writes,:snapshot,:kind,:mode)}.each do |key,rows|
  next unless key[1]=='once'
  raise "Width growth #{key}" unless rows.size==2&&rows.map{|x|x[:candidate]}.uniq.size==1
 end if policy=='improvement'
 r[:deltas].group_by{|x|x.values_at(:width,:writes,:snapshot,:kind)}.each{|key,rows|raise "VM/JIT mismatch #{key}" unless rows.size==2&&rows.map{|x|x.values_at(:baseline,:candidate)}.uniq.size==1}
 r[:status]='passed';save.call;puts '48 dense allocation pairs / 72 semantic cases passed'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
