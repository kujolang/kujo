# Preserve invalid-index behavior without speculative pending-root snapshots.
require 'json';require 'fileutils';require 'open3';require 'digest'
abort 'Expected BASE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR' unless ARGV.size==4
baseline,candidate,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,rows:[],semantics:[],sources:{},identities:[baseline,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 %w[struct dictionary].product([32,2048],%w[once fresh],[16,32],%w[negated_leaf leaf selector]).each do |shape,width,snapshot,calls,form|
  dir=out+"/#{shape}-#{width}-#{snapshot}-#{calls}-#{form}";FileUtils.mkdir_p(dir)
  row=shape=='struct' ? 'state.row' : 'state["row"]'
  root=shape=='struct' ? "struct State { row: array }\nmut state := State { row: range(0, #{width}) }" : "mut state := {\"row\": range(0, #{width})}"
  invalid = form == 'negated_leaf' ? row+'[-state]' : form == 'leaf' ? row+'[state]' : 'state[state][-1]'
  mod="#{root}\nexport func snapshot() { return state }\nexport func fail() { #{row}[0] += 1\nreturn #{invalid} }\n"
  at=->(name){row.sub('state',name)+'[0]'}
  src="from owner import snapshot, fail\nlet original := snapshot()\nmut i := 0\nwhile i < #{calls} { #{snapshot=='fresh' ? 'let saved := snapshot()' : ''}\nmut caught := false\ntry { fail() } except e { caught = true }\nassert(caught)\n#{snapshot=='fresh' ? 'assert_equal('+at.call('saved')+', i)' : ''}\ni += 1 }\nassert_equal(#{at.call('original')}, 0)\nlet current := snapshot()\nassert_equal(#{at.call('current')}, #{calls})\nprint(\"NEGATIVE OPERAND PASS\")\n"
  File.write(dir+'/owner.kujo',mod);path=dir+'/main.kujo';File.write(path,src)
  r[:sources][Digest::SHA256.hexdigest(mod)]=mod;r[:sources][Digest::SHA256.hexdigest(src)]=src
  metadata={form:form,shape:shape,width:width,snapshot:snapshot,calls:calls,module_sha256:Digest::SHA256.hexdigest(mod),source_sha256:Digest::SHA256.hexdigest(src)}
  %w[vm jit].each do |mode|
   pairs=[baseline,candidate].map do |probe|
    stdout,stderr,status=Open3.capture3(probe,mode,path);raise stdout+stderr unless status.success?&&stdout=="NEGATIVE OPERAND PASS\n"
    counts=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless counts
    {requests:counts[1].to_i,bytes:counts[2].to_i}
   end
   r[:rows]<<metadata.merge(mode:mode,baseline:pairs[0],candidate:pairs[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="NEGATIVE OPERAND PASS\n"
   r[:semantics]<<metadata.merge(mode:mode,exit:0);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==96&&r[:semantics].size==144
 r[:regressions]=r[:rows].select{|x|x[:candidate][:bytes]>x[:baseline][:bytes]||x[:candidate][:requests]>x[:baseline][:requests]}
 raise 'Invalid-operand allocation regression' unless r[:regressions].empty?
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:shape,:width,:snapshot,:mode,:form)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};delta={form:key[4],shape:key[0],width:key[1],snapshot:key[2],mode:key[3]}
  %i[baseline candidate].each{|label|delta[label]=%i[requests bytes].to_h{|metric|[metric,(b[label][metric]-a[label][metric])/16.0]}}
  raise 'Added-call regression' unless %i[requests bytes].all?{|metric|delta[:candidate][metric]<=delta[:baseline][metric]}
  delta
 end
 r[:status]='passed';save.call;puts 'Invalid-operand allocation and semantic controls passed'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
