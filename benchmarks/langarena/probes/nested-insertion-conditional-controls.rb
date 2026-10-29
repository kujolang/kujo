# Conditional first existing/new-key writes under conservative observed bounds.
# Both branch outcomes are explicit controls; no tolerance or discarded samples.
require 'json';require 'digest';require 'open3';require 'fileutils'
abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT' unless ARGV.size == 4
baseline,candidate,binary,o=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(o)
[baseline,candidate,binary].each{|p|raise 'Missing executable '+p unless File.executable?(p)}
FileUtils.mkdir_p(o)
probes={baseline:baseline,candidate:candidate}
r={status:'running',identities:[baseline,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,rows:[],semantics:[],sources:{}}
save=->{File.write(o+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
%w[dict struct array].product([['existing_branch',5],['existing_branch',64],['new_branch',5],['new_branch',64]],[2,128],%w[none fresh],[1,2]).each do |shape,pattern,width,snapshot,calls|
 style,writes=pattern
 d=o+"/fixtures/#{shape}-#{style}-#{writes}-#{width}-#{snapshot}-#{calls}";FileUtils.mkdir_p(d)
 root,row=case shape
 when 'dict'; ['mut state := {"row": seed}','state["row"]']
 when 'struct'; ['struct State { row: dict } mut state := State { row: seed }','state.row']
 else; ['mut state := [seed]','state[0]']
 end
 params='select'
 arguments=style=='existing_branch' ? '1' : '0'
 edits=(0...writes).map{|j| key=style=='parameters' ? "k#{j}" : (width+j).to_s; "#{row}[#{key}] = #{key}"}.join(' ')
 owner="mut seed := {} mut k := 0 while k < #{width} { seed[k] = 0 k += 1 }\n#{root}\nseed = null\nexport func snapshot() { return state }\nexport func edit(#{params}) { if select > 0 { #{row}[0] = 0 } else { #{row}[#{width}] = #{width} } #{edits} return #{writes} }\n"
 saved=row.sub('state','saved');final=row.sub('state','final')
 src="from owner import edit, snapshot\nmut i := 0 while i < #{calls} {\n"
 src+="let saved := snapshot()\n" if snapshot=='fresh'
 src+="assert_equal(edit(#{arguments}), #{writes})\n"
 if snapshot=='fresh'
  src+="if i == 0 { assert_equal(len(#{saved}), #{width}) assert_equal(has_key(#{saved}, #{width}), 0) } else { assert_equal(len(#{saved}), #{width+writes}) }\nmut j := 0 while j < #{width} { assert_equal(#{saved}[j], 0) j += 1 } if i > 0 { while j < #{width+writes} { assert_equal(#{saved}[j], j) j += 1 } }\n"
 end
 src+="i += 1 }\nlet final := snapshot()\nassert_equal(len(#{final}), #{width+writes})\nmut k := 0 while k < #{width} { assert_equal(#{final}[k], 0) k += 1 }\nwhile k < #{width+writes} { assert_equal(#{final}[k], k) k += 1 }\nprint(\"NESTED MIXED INSERTION PASS\")\n"
  File.write(d+'/owner.kujo',owner);File.write(d+'/main.kujo',src)
  r[:sources][Digest::SHA256.hexdigest(owner)]=owner;r[:sources][Digest::SHA256.hexdigest(src)]=src
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',d+'/main.kujo','--untrusted'];args<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="NESTED MIXED INSERTION PASS\n"
   r[:semantics]<<{shape:shape,style:style,writes:writes,width:width,snapshot:snapshot,calls:calls,mode:mode,source_sha256:Digest::SHA256.hexdigest(src),module_sha256:Digest::SHA256.hexdigest(owner),exit:status.exitstatus};save.call
  end
  %w[vm jit].each do |mode|
   row={shape:shape,style:style,writes:writes,width:width,snapshot:snapshot,calls:calls,mode:mode,source_sha256:Digest::SHA256.hexdigest(src),module_sha256:Digest::SHA256.hexdigest(owner)}
   row[:samples]={baseline:[],candidate:[]}
   repeats=width==2 && writes==5 ? 12 : 2
   repeats.times do |repeat|
    order=repeat.even? ? probes.keys : probes.keys.reverse
    order.each do |kind|
     stdout,stderr,status=Open3.capture3(probes.fetch(kind),mode,d+'/main.kujo');raise stdout+stderr unless status.success?&&stdout=="NESTED MIXED INSERTION PASS\n"
     m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
     row[:samples][kind]<<{requests:m[1].to_i,bytes:m[2].to_i}
    end
   end
   row[:baseline]=%i[requests bytes].to_h{|m|[m,row[:samples][:baseline].map{|x|x[m]}.min]}
   row[:candidate]=%i[requests bytes].to_h{|m|[m,row[:samples][:candidate].map{|x|x[m]}.max]}
   r[:rows]<<row;save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==192 && r[:semantics].size==288
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:slopes]=r[:rows].group_by{|x|x.values_at(:shape,:style,:writes,:width,:snapshot,:mode)}.flat_map do |group,rows|
  rows.sort_by{|x|x[:calls]}.each_slice(2).map do |a,b|
   n=b[:calls]-a[:calls];h={group:group,calls:[a[:calls],b[:calls]]}
   h[:baseline]=%i[requests bytes].to_h{|m|[m,(b[:samples][:baseline].map{|x|x[m]}.min-a[:samples][:baseline].map{|x|x[m]}.max).to_f/n]}
   h[:candidate]=%i[requests bytes].to_h{|m|[m,(b[:samples][:candidate].map{|x|x[m]}.max-a[:samples][:candidate].map{|x|x[m]}.min).to_f/n]}
   h
  end
 end
 r[:slope_regressions]=r[:slopes].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:probe_executions]=r[:rows].sum{|x|x[:samples].values.sum(&:size)}
 r[:comparison]='Observed candidate maximum vs baseline minimum totals; candidate upper vs baseline lower added-call slopes. Finite samples, no tolerance or speed claim.'
 r[:identities].each{|p,h|raise 'Executable drift' unless Digest::SHA256.file(p).hexdigest==h}
 r[:status]=r[:regressions].empty?&&r[:slope_regressions].empty? ? 'passed' : 'rejected';save.call
 puts JSON.generate({status:r[:status],regressions:r[:regressions].size})
 raise 'Insertion allocation regression' unless r[:status]=='passed'
rescue=>e;r[:status]='failed' unless r[:status]=='rejected';r[:error]=e.message;save.call;raise
end
