require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY NEW_OUTPUT_DIR' unless ARGV.size==4
base,candidate,binary,out=ARGV.map{|p|File.expand_path(p)};raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[]};save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 %w[string insert].product([3,2048],[16,32]).each do |pattern,width,calls|
  folder=out+"/#{pattern}-#{width}-#{calls}";FileUtils.mkdir_p(folder)
  tail=pattern=='string' ? 'state[2] = "changed"' : "state[n + #{width}] = n"
  module_source="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func edit(n) { state[0] = n state[1] = n #{tail} return state[0] }\n"
  check=pattern=='string' ? 'assert_equal(snapshot()[2], "changed")' : "assert_equal(snapshot()[n + #{width}], n)"
  source="from owner import snapshot, edit\nlet original := snapshot()\nmut n := 1\nwhile n <= #{calls} { assert_equal(edit(n), n) assert_equal(snapshot()[1], n) #{check} n += 1 }\nassert_equal(original[0], 0) assert_equal(original[1], 1) assert_equal(original[2], 2)\nprint(\"FALLBACK PASS\")\n"
  File.write(folder+'/owner.kujo',module_source);path=folder+'/main.kujo';File.write(path,source)
  meta={pattern:pattern,width:width,calls:calls,module_sha256:Digest::SHA256.hexdigest(module_source),source_sha256:Digest::SHA256.hexdigest(source)}
  %w[vm jit].each do |mode|
   pair=[base,candidate].map do |probe|
    o,e,s=Open3.capture3(probe,mode,path);raise o+e unless s.success?&&o=="FALLBACK PASS\n";m=e.match(/requests=(\d+) requested_bytes=(\d+)/);raise e unless m;{requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm';o,e,s=Open3.capture3(*args);raise o+e unless s.success?&&o=="FALLBACK PASS\n";r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==16&&r[:semantics].size==24
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:status]=r[:regressions].empty? ? 'passed' : 'regression';save.call
 puts "Fallback controls #{r[:status]}:#{r[:regressions].size} regressed pairs"
 raise 'Fallback allocation regression' unless r[:regressions].empty?
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
