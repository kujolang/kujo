# Repository-maintenance allocation regression gate; no elapsed-speed claim.
require 'json';require 'digest';require 'open3'
abort 'Expected BASELINE_PROBE CANDIDATE_PROBE NEW_OUTPUT_DIR' unless ARGV.size == 3
baseline, candidate, d = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(d)
[baseline, candidate].each { |p| raise 'Missing probe ' + p unless File.executable?(p) }
Dir.mkdir(d)
fixtures=d
[128,2048].product([2,4],%w[nested extracted]).each do |width,rounds,form|
 init=form=='nested' ? 'mut state := input' : 'let src := input[0] mut dst := input[1]'
 body=form=='nested' ? 'state[1][i] = state[0][i]' : 'dst[i] = src[i]'
 result=form=='nested' ? 'state' : '[src, dst]'
 source="func copy_rows(input, rounds) { #{init}\nmut r := 0\nwhile r < rounds { mut i := 0\nwhile i < #{width} { #{body}\ni += 1 }\nr += 1 }\nreturn #{result} }\nlet original := [range(0, #{width}), range(#{width}, #{2*width})]\nlet result := copy_rows(original, #{rounds})\nassert_equal(original[1][0], #{width})\nassert_equal(original[1][-1], #{2*width-1})\nassert_equal(result[0], original[0])\nassert_equal(result[1], original[0])\nprint(\"ROW ACCESS PASS\")\n"
 File.write(d+"/#{form}-#{width}-#{rounds}.kujo",source)
end
r={status:'running',pid:Process.pid,acceptance:'VM nested writes must remove the reproduced two requests/320 bytes per write at both widths; extracted controls must not increase. JIT paired results are recorded separately; no exact noisy compile-total equality claim.',rows:[],sources:{},identities:{}}
[baseline,candidate,__FILE__].each{|p|r[:identities][p]=Digest::SHA256.file(p).hexdigest}
save=->{File.write(d+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 [128,2048].product([2,4],%w[nested extracted],%w[vm jit]).each do |width,rounds,form,mode|
  path=fixtures+"/#{form}-#{width}-#{rounds}.kujo";r[:sources][path]=Digest::SHA256.file(path).hexdigest
  %w[baseline candidate].each do |version|
   probe=version=='baseline' ? baseline : candidate
   out,err,status=Open3.capture3(probe,mode,path);raise out+err unless status.success?&&out=="ROW ACCESS PASS\n"
   m=err.match(/requests=(\d+) requested_bytes=(\d+)/);raise err unless m
   r[:rows]<<{width:width,rounds:rounds,form:form,mode:mode,version:version,requests:m[1].to_i,bytes:m[2].to_i};save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==32
 r[:slopes]=r[:rows].group_by{|x|x.values_at(:width,:form,:mode,:version)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:rounds]};{width:key[0],form:key[1],mode:key[2],version:key[3],requests:(b[:requests]-a[:requests])/(2.0*key[0]),bytes:(b[:bytes]-a[:bytes])/(2.0*key[0])}
 end
 [128,2048].each do |width|
  get=->(form,version){r[:slopes].find{|x|x.values_at(:width,:form,:mode,:version)==[width,form,'vm',version]}}
  b=get.call('nested','baseline');c=get.call('nested','candidate');e=get.call('extracted','candidate');eb=get.call('extracted','baseline')
  raise 'VM target did not remove reproduced cost' unless b[:requests]-c[:requests]>=2&&b[:bytes]-c[:bytes]>=320
  raise 'VM extracted regression' unless e[:requests]<=eb[:requests]&&e[:bytes]<=eb[:bytes]
 end
 r[:rows].select{|x|x[:mode]=='vm'&&x[:version]=='candidate'}.each do |c|
  b=r[:rows].find{|x|x[:version]=='baseline'&&x.values_at(:width,:rounds,:form,:mode)==c.values_at(:width,:rounds,:form,:mode)}
  raise 'VM total regression' unless c[:requests]<=b[:requests]&&c[:bytes]<=b[:bytes]
 end
 r[:identities].merge(r[:sources]).each{|p,h|raise 'Artifact drift '+p unless Digest::SHA256.file(p).hexdigest==h}
 r[:status]='passed';save.call
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
