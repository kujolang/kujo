require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY OUTPUT' unless ARGV.size==4
base,candidate,binary,out=ARGV.map{|p|File.expand_path(p)};raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
cases={'computed_int'=>['return state[0 + 0]',false], 'computed_string'=>['return state["" + "0"]',false], 'none_write'=>['state[2] = None return state[0]',false]}
{'null'=>'null','float'=>'1.5','bool'=>'true','bytes'=>'payload'}.each{|name,expr|cases['read_'+name]=["return state[#{expr}]",true];cases['write_'+name]=["state[#{expr}] = n return state[0]",true]}
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[]};save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 cases.each do |pattern,(tail,error)|
 [3,2048].product([16,32]).each do |width,calls|
  dir=out+"/#{pattern}-#{width}-#{calls}";FileUtils.mkdir_p(dir)
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func edit(n, payload) { state[0] = n state[1] = n #{tail} }\n"
  call=error ? 'mut caught := false try { edit(n, payload) } except e { caught = true } assert(caught)' : 'assert_equal(edit(n, payload), n)'
  check=pattern=='none_write' ? 'assert_equal(snapshot()[2], None)' : 'assert_equal(snapshot()[2], 2)'
  source="from owner import snapshot, edit\nlet original := snapshot()\nlet payload := bytes([1,2])\nmut n := 1\nwhile n <= #{calls} { #{call} assert_equal(snapshot()[0], n) assert_equal(snapshot()[1], n) #{check} n += 1 }\nassert_equal(original[0], 0) assert_equal(original[1], 1) assert_equal(original[2], 2)\nprint(\"INDEX CONTROL PASS\")\n"
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,source)
  meta={pattern:pattern,width:width,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(source)}
  %w[vm jit].each do |mode|
   pair=[base,candidate].map do |probe|
    o,e,s=Open3.capture3(probe,mode,path);raise o+e unless s.success?&&o=="INDEX CONTROL PASS\n";m=e.match(/requests=(\d+) requested_bytes=(\d+)/);raise e unless m;{requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm';o,e,s=Open3.capture3(*args);raise o+e unless s.success?&&o=="INDEX CONTROL PASS\n";r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
  end
 end
 end
 raise 'Coverage' unless r[:rows].size==88&&r[:semantics].size==132
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:status]=r[:regressions].empty? ? 'passed' : 'regression';save.call
 puts "Index controls #{r[:status]}:#{r[:regressions].size} regressed pairs"
 raise 'index allocation regression' unless r[:regressions].empty?
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
