# Cold initial insertion gate without an initial user snapshot.
# Multiple new keys must not regress empty/tiny-owner allocation traffic.
# All snapshot and output assertions run in VM, interpreter and JIT.
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
 [2,64].product([0,1,2,128,2048],%w[none fresh],[1,2,16,32]).each do |writes,width,snapshot,calls|
  d=o+"/fixtures/#{writes}-#{width}-#{snapshot}-#{calls}";FileUtils.mkdir_p(d)
  owner="mut data := {}\nmut k := 0 while k < #{width} { data[k] = 0 k += 1 }\nexport func snapshot() { return data }\nexport func edit(k) { #{writes == 2 ? "data[k] = k data[k + 1] = k + 1" : "mut j := 0 while j < 64 { data[k + j] = k + j j += 1 }"} return k }\n"
  src="from owner import edit, snapshot\nmut i := 0 while i < #{calls} {\n"
  src+="let saved := snapshot()\n" if snapshot=='fresh'
  src+="assert_equal(edit(#{width} + #{writes} * i), #{width} + #{writes} * i)\n"
  src+="assert_equal(has_key(saved, #{width} + #{writes} * i), 0) assert_equal(len(saved), #{width} + #{writes} * i)\n" if snapshot=='fresh'
  src+="i += 1 }\nlet final := snapshot()\nassert_equal(len(final), #{width+writes*calls})\nmut k := 0 while k < #{width} { assert_equal(final[k], 0) k += 1 }\nwhile k < #{width+writes*calls} { assert_equal(final[k], k) k += 1 }\nprint(\"PAIR INSERTION PASS\")\n"
  File.write(d+'/owner.kujo',owner);File.write(d+'/main.kujo',src)
  r[:sources][Digest::SHA256.hexdigest(owner)]=owner;r[:sources][Digest::SHA256.hexdigest(src)]=src
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',d+'/main.kujo','--untrusted'];args<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*args);raise stdout+stderr unless status.success?&&stdout=="PAIR INSERTION PASS\n"
   r[:semantics]<<{writes:writes,width:width,snapshot:snapshot,calls:calls,mode:mode,source_sha256:Digest::SHA256.hexdigest(src),module_sha256:Digest::SHA256.hexdigest(owner),exit:status.exitstatus};save.call
  end
  %w[vm jit].each do |mode|
   row={writes:writes,width:width,snapshot:snapshot,calls:calls,mode:mode,source_sha256:Digest::SHA256.hexdigest(src),module_sha256:Digest::SHA256.hexdigest(owner)}
   probes.each do |kind,probe|
    stdout,stderr,status=Open3.capture3(probe,mode,d+'/main.kujo');raise stdout+stderr unless status.success?&&stdout=="PAIR INSERTION PASS\n"
    m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
    row[kind]={requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<row;save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==160 && r[:semantics].size==240
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:slopes]=r[:rows].group_by{|x|x.values_at(:writes,:width,:snapshot,:mode)}.flat_map do |group,rows|
  rows.sort_by{|x|x[:calls]}.each_slice(2).map do |a,b|
   n=b[:calls]-a[:calls];h={group:group,calls:[a[:calls],b[:calls]]}
   %i[baseline candidate].each{|kind|h[kind]=%i[requests bytes].to_h{|m|[m,(b[kind][m]-a[kind][m]).to_f/n]}};h
  end
 end
 r[:slope_regressions]=r[:slopes].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 r[:identities].each{|p,h|raise 'Executable drift' unless Digest::SHA256.file(p).hexdigest==h}
 r[:status]=r[:regressions].empty?&&r[:slope_regressions].empty? ? 'passed' : 'rejected';save.call
 puts JSON.generate({status:r[:status],regressions:r[:regressions].size})
 raise 'Insertion allocation regression' unless r[:status]=='passed'
rescue=>e;r[:status]='failed' unless r[:status]=='rejected';r[:error]=e.message;save.call;raise
end
