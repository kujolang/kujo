# Compare struct construction boundaries with parameter payloads; allocation only.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected PROBE BINARY NEW_OUTPUT' unless ARGV.size==3
probe,binary,out=ARGV.map{|p|File.expand_path(p)};raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',scope:'Struct replacement literal versus argument; independent width/call slopes, not elapsed speed.',identities:[probe,binary,__FILE__].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},sources:{},rows:[],semantics:[]};save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 %w[root nested].product(%w[literal parameter],[2,2048],%w[once fresh],[16,32]).each do |shape,form,width,snapshot,calls|
  d=out+'/'+[shape,form,width,snapshot,calls].join('-');FileUtils.mkdir_p(d)
  root=shape=='root' ? 'mut state := seed' : 'mut state := {"branch": {"row": seed}}'
  row=shape=='root' ? 'state' : 'state["branch"]["row"]'
  params=form=='literal' ? 'n' : 'n, payload';replacement=form=='literal' ? 'Item { value: n }' : 'payload'
  owner=<<~KUJO
    struct Item { value: int }
    mut seed := {} mut j := 0 while j < #{width} { seed[j] = Item { value: 0 } j += 1 }
    #{root} seed = null
    export func snapshot() { return state }
    export func edit(#{params}) { #{row}[0] = #{replacement} return n }
  KUJO
  original,final,saved=%w[original final saved].map{|name|row.sub('state',name)}
  args=form=='literal' ? 'i + 1' : 'i + 1, Item { value: i + 1 }'
  source="from owner import snapshot, edit\nstruct Item { value: int }\nlet original := snapshot()\nmut i := 0 while i < #{calls} {\n"
  source+="let saved := snapshot()\n" if snapshot=='fresh'
  source+="assert_equal(edit(#{args}), i + 1)\n"
  if snapshot=='fresh'
   source+="assert_equal(len(#{saved}), #{width}) mut j := 0 while j < #{width} { if j == 0 { assert_equal(#{saved}[j].value, i) } else { assert_equal(#{saved}[j].value, 0) } j += 1 }\n"
  end
  source+="i += 1 }\nlet final := snapshot()\nassert_equal(len(#{original}), #{width}) assert_equal(len(#{final}), #{width})\nmut j := 0 while j < #{width} { assert_equal(#{original}[j].value, 0) if j == 0 { assert_equal(#{final}[j].value, #{calls}) } else { assert_equal(#{final}[j].value, 0) } j += 1 }\nprint(\"STRUCT REPLACEMENT PASS\")\n"
  File.write(d+'/main.kujo',source);File.write(d+'/owner.kujo',owner)
  sh=Digest::SHA256.hexdigest(source);mh=Digest::SHA256.hexdigest(owner);r[:sources][sh]=source;r[:sources][mh]=owner
  meta={shape:shape,form:form,width:width,snapshot:snapshot,calls:calls,source_sha256:sh,module_sha256:mh}
  %w[vm interpreter jit].each do |mode|
   argv=[binary,'run',d+'/main.kujo','--untrusted','--scheduler-no-timeout'];argv<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*argv);raise stdout+stderr unless status.success?&&stdout=="STRUCT REPLACEMENT PASS\n"
   r[:semantics]<<meta.merge(mode:mode,exit:status.exitstatus);save.call
  end
  %w[vm jit].each do |mode|
   stdout,stderr,status=Open3.capture3(probe,mode,d+'/main.kujo');raise stdout+stderr unless status.success?&&stdout=="STRUCT REPLACEMENT PASS\n"
   m=stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/);raise stderr unless m
   r[:rows]<<meta.merge(mode:mode,requests:m[1].to_i,bytes:m[2].to_i);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==64&&r[:semantics].size==96
 r[:slopes]=r[:rows].group_by{|x|x.values_at(:shape,:form,:width,:snapshot,:mode)}.map do |group,rows|
  a,b=rows.sort_by{|x|x[:calls]};{group:group,calls:[a[:calls],b[:calls]],requests:(b[:requests]-a[:requests]).fdiv(b[:calls]-a[:calls]),bytes:(b[:bytes]-a[:bytes]).fdiv(b[:calls]-a[:calls])}
 end
 r[:identities].each{|p,h|raise 'Artifact drift' unless Digest::SHA256.file(p).hexdigest==h}
 r[:status]='characterized';save.call
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
