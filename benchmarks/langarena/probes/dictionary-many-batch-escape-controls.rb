require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASE_PROBE CANDIDATE_PROBE BINARY OUTPUT' unless ARGV.size==4
baseline,candidate,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[baseline,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[],sources:{}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 %w[int float bool null].product(%w[root snapshot_then_write wrapped],[2,2048],%w[once fresh],[2,4,16,64],[16,32]).each do |kind,pattern,width,snapshot,writes,calls|
  next if writes > width
  dir=out+"/#{kind}-#{pattern}-#{width}-#{snapshot}-#{writes}-#{calls}";FileUtils.mkdir_p(dir)
  first={'int'=>'n','float'=>'n + 0.5','bool'=>'n % 2 == 0','null'=>'null'}[kind]
  second={'int'=>'n + 1','float'=>'n + 1.5','bool'=>'n % 2 != 0','null'=>'null'}[kind]
  ending=pattern=='snapshot_then_write' ? "let saved := state\nstate[0] = #{second}\nreturn saved" : pattern=='wrapped' ? 'return [state]' : 'return state'
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func edit(n) { mut key := 0 while key < #{writes} { state[key] = #{first} key += 1 }\n#{ending} }\n"
  returned=pattern=='wrapped' ? 'returned[0][0]' : 'returned[0]'
  returned_root=pattern=='wrapped' ? 'returned[0]' : 'returned'
  current=pattern=='snapshot_then_write' ? second : first
  before=snapshot=='fresh' ? 'let before := snapshot() let old := before[0] let old_second := before[1]' : ''
  check=snapshot=='fresh' ? 'assert_equal(before[0], old) assert_equal(before[1], old_second)' : ''
  src="from owner import snapshot, edit\nlet original := snapshot()\nmut n := 1\nwhile n <= #{calls} { #{before}\nlet returned := edit(n)\nlet observed := #{returned_root} mut k := 0 while k < #{writes} { assert_equal(observed[k], #{first}) k += 1 }\nassert_equal(#{returned}, #{first})\nassert_equal(type(#{returned}), type(#{first}))\nassert_equal(snapshot()[0], #{current})\nassert_equal(snapshot()[1], #{first})\n#{check}\nn += 1 }\nmut k := 0 while k < #{writes} { assert_equal(original[k], k) k += 1 }\nassert_equal(original[0], 0)\nassert_equal(original[#{width-1}], #{width-1})\nassert_equal(snapshot()[#{width-1}], #{width==2 ? first.gsub(/\bn\b/,calls.to_s) : width-1})\nprint(\"DICTIONARY ESCAPE PASS\")\n"
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,src)
  [mod,src].each{|s|r[:sources][Digest::SHA256.hexdigest(s)]=s}
  meta={kind:kind,pattern:pattern,width:width,snapshot:snapshot,writes:writes,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(src)}
  %w[vm jit].each do |mode|
   pair=[baseline,candidate].map do |probe|
    stdout,stderr,status=Open3.capture3(probe,mode,path);raise stdout+stderr unless status.success?&&stdout=="DICTIONARY ESCAPE PASS\n"
    m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
    {requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="DICTIONARY ESCAPE PASS\n"
   r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==480&&r[:semantics].size==720
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 raise 'Dictionary escape allocation regression' unless r[:regressions].empty?
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:kind,:pattern,:width,:snapshot,:writes,:mode)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};delta={kind:key[0],pattern:key[1],width:key[2],snapshot:key[3],writes:key[4],mode:key[5]}
  %i[baseline candidate].each{|label|delta[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/16.0]}}
  raise "Escape added-call regression #{key}" unless %i[requests bytes].all?{|m|delta[:candidate][m]<=delta[:baseline][m]}
  delta
 end
 r[:status]='passed';save.call;puts 'Dictionary escape controls passed:480 pairs /720 semantics'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
