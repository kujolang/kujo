# Exact compound-update and whole-root error controls for inline numeric pairs.
require 'json'; require 'digest'; require 'fileutils'; require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY OUTPUT [improvement|control]' unless (4..5).cover?(ARGV.size)
base,candidate,binary,out=ARGV.first(4).map{|p|File.expand_path(p)};policy=ARGV[4]||'improvement'
raise 'Policy' unless %w[improvement control].include?(policy)
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',policy:policy,runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[]};save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 %w[compound self_index].product([2,128,2048],%w[once fresh],[16,32]).each do |pattern,width,snapshot,calls|
  dir=out+"/#{pattern}-#{width}-#{snapshot}-#{calls}";FileUtils.mkdir_p(dir)
  result=pattern=='compound' ? 'return state[0]' : 'return state[state]'
  mod="mut state := {\"0\": 0}\nmut k := 1\nwhile k < #{width} { state[k] = k k += 1 }\nexport func snapshot() { return state }\nexport func edit() { state[0] += 1 state[1] += 1 #{result} }\n"
  invoke=pattern=='compound' ? 'assert_equal(edit(), n)' : 'mut caught := false try { edit() } except e { caught = true } assert(caught)'
  save_snapshot=snapshot=='fresh' ? 'let saved := snapshot()' : ''
  check_snapshot=snapshot=='fresh' ? 'assert_equal(saved[0], n - 1) assert_equal(saved[1], n)' : ''
  source="from owner import snapshot, edit\nlet original := snapshot()\nmut n := 1\nwhile n <= #{calls} { #{save_snapshot} #{invoke} #{check_snapshot} assert_equal(snapshot()[0], n) assert_equal(snapshot()[1], n + 1) n += 1 }\nassert_equal(original[0], 0) assert_equal(original[1], 1)\nprint(\"PAIR PASS\")\n"
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,source)
  meta={pattern:pattern,width:width,snapshot:snapshot,calls:calls,source_sha256:Digest::SHA256.hexdigest(source),module_sha256:Digest::SHA256.hexdigest(mod)}
  %w[vm jit].each do |mode|
   pair=[base,candidate].map{|probe|o,e,s=Open3.capture3(probe,mode,path);raise o+e unless s.success?&&o=="PAIR PASS\n";m=e.match(/requests=(\d+) requested_bytes=(\d+)/);raise e unless m;{requests:m[1].to_i,bytes:m[2].to_i}}
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm';o,e,s=Open3.capture3(*args);raise o+e unless s.success?&&o=="PAIR PASS\n";r[:semantics]<<meta.merge(mode:mode,exit:s.exitstatus);save.call
  end
 end
 expected=%w[compound self_index].product([2,128,2048],%w[once fresh],[16,32])
 raise 'Exact allocations' unless r[:rows].map{|x|x.values_at(:pattern,:width,:snapshot,:calls,:mode)}.sort==expected.product(%w[vm jit]).map{|a,m|a+[m]}.sort
 raise 'Exact semantics' unless r[:semantics].map{|x|x.values_at(:pattern,:width,:snapshot,:calls,:mode)}.sort==expected.product(%w[vm interpreter jit]).map{|a,m|a+[m]}.sort
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:pattern,:width,:snapshot,:mode)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};x=%i[pattern width snapshot mode].zip(key).to_h
  %i[baseline candidate].each{|label|x[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/16.0]}};x
 end
 raise 'Total allocation regression' unless r[:regressions].empty?
 raise 'Slope regression' unless r[:deltas].all?{|x|%i[requests bytes].all?{|m|x[:candidate][m]<=x[:baseline][m]}}
 if policy=='improvement'
  target=r[:deltas].select{|x|x[:pattern]=='compound'&&x[:snapshot]=='once'&&x[:width]>=128}
  raise 'No target improvement' unless target.size==4&&target.all?{|x|x[:candidate][:bytes]<x[:baseline][:bytes]}
  raise 'Width growth' unless target.group_by{|x|x[:mode]}.values.all?{|rows|rows.map{|x|x[:candidate]}.uniq.size==1}
 end
 r[:status]='passed';save.call;puts 'Pair gate passed:48 allocation pairs /72 semantics'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
