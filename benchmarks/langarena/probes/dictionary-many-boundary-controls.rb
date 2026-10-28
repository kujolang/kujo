require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY OUTPUT' unless ARGV.size==4
base,candidate,binary,out=ARGV.map{|p|File.expand_path(p)};raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[]};save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 %w[bytes other_root].product([3,2048],[16,32]).each do |pattern,width,calls|
  dir=out+"/#{pattern}-#{width}-#{calls}";FileUtils.mkdir_p(dir)
  update=pattern=='bytes' ? 'state[2] = payload' : 'other[0] = n'
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nmut other := {\"0\": 0}\nexport func snapshot() { return state }\nexport func other_snapshot() { return other }\nexport func edit(n, payload) { state[0] = n state[1] = n #{update} return state[0] }\n"
  check=pattern=='bytes' ? 'assert_equal(snapshot()[2], payload)' : 'assert_equal(other_snapshot()[0], n)'
  source="from owner import snapshot, other_snapshot, edit\nlet original := snapshot()\nlet original_other := other_snapshot()\nlet payload := bytes([1,2,255])\nmut n := 1\nwhile n <= #{calls} { assert_equal(edit(n, payload), n) assert_equal(snapshot()[1], n) #{check} n += 1 }\nassert_equal(original[0], 0) assert_equal(original[1], 1) assert_equal(original[2], 2) assert_equal(original_other[0], 0)\nprint(\"BOUNDARY PASS\")\n"
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,source)
  meta={pattern:pattern,width:width,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(source)}
  %w[vm jit].each do |mode|
   pair=[base,candidate].map do |probe|
    o,e,s=Open3.capture3(probe,mode,path);raise o+e unless s.success?&&o=="BOUNDARY PASS\n";m=e.match(/requests=(\d+) requested_bytes=(\d+)/);raise e unless m;{requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm';o,e,s=Open3.capture3(*args);raise o+e unless s.success?&&o=="BOUNDARY PASS\n";r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==16&&r[:semantics].size==24
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:status]=r[:regressions].empty? ? 'passed' : 'regression';save.call
 puts "Boundary controls #{r[:status]}:#{r[:regressions].size} regressed pairs"
 raise 'boundary allocation regression' unless r[:regressions].empty?
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
