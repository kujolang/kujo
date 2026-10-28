# Isolate path-depth copying from repeated writes and required fresh snapshots.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected ACCEPTED_PROBE ACCEPTED_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable '+p unless File.executable?(p) }
FileUtils.mkdir_p(output)
r={status:'running',scope:'Accepted-runtime depth/width/repeated-write isolation; allocation traffic, not timing',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,probe_sha256:Digest::SHA256.file(probe).hexdigest,binary_sha256:Digest::SHA256.file(binary).hexdigest,rows:[],semantics:[],sources:{}}
save=->{File.write(output+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 %w[dictionary struct array].product(%w[array dictionary],[2,4],[128,2048],[1,8],%w[once fresh]).each do |owner,leaf,depth,width,writes,snapshot|
  dir="#{output}/#{owner}-#{leaf}-#{depth}-#{width}-#{writes}-#{snapshot}";FileUtils.mkdir_p(dir)
  seed=leaf=='array' ? "mut seed := range(0, #{width})" : "mut seed := {}\nmut k := 0\nwhile k < #{width} { seed[k] = k k += 1 }"
  inner='seed';(depth-2).times{inner='{"inner": '+inner+'}'}
  init,row=case owner
   when 'dictionary';['mut state := {"row": '+inner+'}','state["row"]']
   when 'struct';["struct State { row: any }\nmut state := State { row: "+inner+' }','state.row']
   when 'array';['mut state := ['+inner+']','state[0]']
  end
  row+='["inner"]'*(depth-2)
  mod="#{seed}\n#{init}\nseed = null\nexport func snapshot() { return state }\nexport func edit() {\nmut j := 0\nwhile j < #{writes} { #{row}[0] += 1 j += 1 }\nreturn #{row}[0]\n}\n"
  File.write(dir+'/owner.kujo',mod);r[:sources][Digest::SHA256.hexdigest(mod)]=mod
  [16,32].each do |calls|
   at=->(name,index){row.sub('state',name)+"[#{index}]"}
   before=snapshot=='fresh' ? "let saved := snapshot()\n" : ''
   check=snapshot=='fresh' ? "assert_equal(#{at.call('saved',0)}, i * #{writes})\nassert_equal(#{at.call('saved',1)}, 1)" : ''
   source="from owner import snapshot, edit\nlet original := snapshot()\nmut i := 0\nwhile i < #{calls} { #{before}assert_equal(edit(), (i + 1) * #{writes})\n#{check}\ni += 1 }\nlet final := snapshot()\nassert_equal(#{at.call('final',0)}, #{calls*writes})\nmut k := 0\nwhile k < #{width} { assert_equal(#{at.call('original','k')}, k)\nif k > 0 { assert_equal(#{at.call('final','k')}, k) }\nk += 1 }\nprint(\"DEEP COLLECTION PASS\")\n"
   path=dir+"/calls-#{calls}.kujo";File.write(path,source);r[:sources][Digest::SHA256.hexdigest(source)]=source
   meta={owner:owner,leaf:leaf,depth:depth,width:width,writes:writes,snapshot:snapshot,calls:calls,source_sha256:Digest::SHA256.hexdigest(source),module_sha256:Digest::SHA256.hexdigest(mod)}
   %w[vm jit].each do |mode|
    stdout,stderr,status=Open3.capture3(probe,mode,path);raise "#{meta} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout=="DEEP COLLECTION PASS\n"
    m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
    r[:rows]<<meta.merge(mode:mode,requests:m[1].to_i,bytes:m[2].to_i);save.call
   end
   %w[vm interpreter jit].each do |mode|
    args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
    stdout,stderr,status=Open3.capture3(*args);raise "#{meta} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout=="DEEP COLLECTION PASS\n"
    r[:semantics]<<meta.merge(mode:mode,exit:status.exitstatus);save.call
   end
  end
 end
 raise 'Coverage' unless r[:rows].size==384 && r[:semantics].size==576
 keys=%i[owner leaf depth width writes snapshot mode]
 r[:deltas]=r[:rows].group_by{|row|keys.map{|k|row[k]}}.map do |key,rows|
  a,b=rows.sort_by{|row|row[:calls]};keys.zip(key).to_h.merge(requests_per_added_call:(b[:requests]-a[:requests])/16.0,bytes_per_added_call:(b[:bytes]-a[:bytes])/16.0)
 end
 r[:status]='characterized';save.call
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
