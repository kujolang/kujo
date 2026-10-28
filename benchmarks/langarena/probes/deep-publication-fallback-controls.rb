# Deep array/dictionary escapes, shared prefixes and invalid operands; allocation control only.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected BASE_PROBE CANDIDATE_PROBE BINARY OUTPUT' unless ARGV.size==4
baseline,candidate,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[baseline,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[],sources:{}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
actions={
 'other_key'=>'state[0][1] = n + 1 return state[0][0]',
 'new_key'=>'state[0]["new"] = n + 1 return state[0][0]',
 'callback'=>'return observe()',
 'root'=>'return state',
 'wrapped'=>'return [state]',
 'row'=>'return state[0]',
 'snapshot_then_write'=>'let saved := state state[0][0] = n + 1 return saved',
 'other_row'=>'state[1][0] = n + 1 return state[1][0]',
 'shared_row'=>'return state[1][0]',
 'shared_prefix'=>'return state[1][0]',
 'nonscalar'=>'state[0][0] = [n] return state[0][0][0]',
 'selector'=>'return state[state][0]',
 'neg_selector'=>'return state[-state][0]',
 'leaf'=>'return state[0][state]',
 'neg_leaf'=>'return state[0][-state]',
 'overflow'=>'let minimum := -9223372036854775807 - 1 return state[-minimum][0]'
}
begin
 actions.keys.product(%w[array dictionary],[32,2048],%w[once fresh],[16,32]).each do |form,leaf,width,snapshot,calls|
  dir=out+"/#{form}-#{leaf}-#{width}-#{snapshot}-#{calls}";FileUtils.mkdir_p(dir)
  seed="mut seed := {}\nmut k := 0\nwhile k < #{width} { seed[k] = k k += 1 }\n"
seed="mut seed := range(0, #{width})\n" if leaf=='array'
root=seed+case form
  when 'shared_prefix' then 'mut branch := {"inner": seed} mut state := [branch, branch] branch = null'
  when 'shared_row' then 'mut state := [{"inner": seed}, {"inner": seed}]'
  else 'mut state := [{"inner": seed}, {"inner": {"0": 99}}]'
end+"\nseed = null"
  mod="#{root}\nfunc observe() { return state[0][0] }\nexport func snapshot() { return state }\nexport func edit(n) { state[0][0] = n\n#{actions.fetch(form)} }\n"
  mod=mod.gsub(/state\[(0|1|state|-state|-minimum)\]/) { |access| access+'["inner"]' }
  error=%w[selector neg_selector leaf neg_leaf overflow].include?(form) || (leaf=='array' && form=='new_key')
  returned=case form
  when 'root','snapshot_then_write' then 'returned[0]["inner"][0]'
  when 'wrapped' then 'returned[0][0]["inner"][0]'
  when 'row' then 'returned[0]'
  else 'returned'
  end
  expected=form=='other_row' ? 'n + 1' : %w[shared_row shared_prefix].include?(form) ? '0' : 'n'
  invoke=error ? 'mut caught := false try { edit(n) } except e { caught = true } assert(caught)' : "let returned := edit(n) assert_equal(#{returned}, #{expected})"
  current=form=='nonscalar' ? 'snapshot()[0][0][0]' : 'snapshot()[0][0]'
  current_expected=form=='snapshot_then_write' ? 'n + 1' : 'n'
  saved=snapshot=='fresh' ? 'let before := snapshot() let old := before[0][0]' : ''
  check=snapshot=='fresh' ? 'assert_equal(before[0][0], old)' : ''
  source="from owner import snapshot, edit\nlet original := snapshot()\nmut n := 1\nwhile n <= #{calls} { #{saved}\n#{invoke}\nassert_equal(#{current}, #{current_expected})\n#{check}\nn += 1 }\nassert_equal(original[0][0], 0)\nassert_equal(original[0][#{width-1}], #{width-1})\nassert_equal(snapshot()[0][#{width-1}], #{width-1})\nprint(\"NESTED DICTIONARY FALLBACK PASS\")\n"
  %w[original[0] before[0] snapshot()[0]].each { |access| source=source.gsub(access,access+'["inner"]') }
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,source)
  [mod,source].each{|s|r[:sources][Digest::SHA256.hexdigest(s)]=s}
  meta={form:form,leaf:leaf,width:width,snapshot:snapshot,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(source)}
  %w[vm jit].each do |mode|
   pair=[baseline,candidate].map do |probe|
    stdout,stderr,status=Open3.capture3(probe,mode,path);raise stdout+stderr unless status.success?&&stdout=="NESTED DICTIONARY FALLBACK PASS\n"
    m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
    {requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="NESTED DICTIONARY FALLBACK PASS\n"
   r[:semantics]<<meta.merge(mode:mode,exit:status.exitstatus);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==512&&r[:semantics].size==768
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 raise 'Deep publication fallback total regression' unless r[:regressions].empty?
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:form,:leaf,:width,:snapshot,:mode)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};delta={form:key[0],leaf:key[1],width:key[2],snapshot:key[3],mode:key[4]}
  %i[baseline candidate].each{|label|delta[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/16.0]}}
  raise "Deep publication fallback added-call regression #{key}" unless %i[requests bytes].all?{|m|delta[:candidate][m]<=delta[:baseline][m]}
  delta
 end
 r[:status]='passed';save.call;puts 'Deep publication fallback controls passed:512 pairs/768 semantics'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
