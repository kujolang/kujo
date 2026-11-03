require 'json'
require 'digest'
require 'base64'
require 'open3'
raise 'usage: UPSTREAM FROZEN_BINARY NEW_OUTPUT' unless ARGV.size==3
arena,binary,root=ARGV.map{|p|File.expand_path(p)}
raise 'output exists' if File.exist?(root)
Dir.mkdir(root)
paths=%w[rust/src/benchmarks/base64_encode.rs rust/src/benchmarks/base64_decode.rs goose/src/base64_encode.goose goose/src/base64_decode.goose]
paths.each do |path|
 text,status=Open3.capture2('git','-C',arena,'show','3c9f84ddaeec834c0631747cb789e8cfd602f993:'+path)
 raise 'upstream drift' unless status.success? && text==File.read(arena+'/'+path)
end
files=[binary,'benchmarks/langarena/port.kujo',__FILE__]+paths.map{|p|arena+'/'+p}
hashes=files.to_h{|f|[f,Digest::SHA256.file(f).hexdigest]}
sizes=[0,1,2,3,4,23,30,255,256,257,1024,500000]
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
sizes.each_with_index do |n,i|
 source += "\nmut e#{i} := base64_prepare(basic_state({\"name\":\"Base64::Encode\",\"size\":#{n}}))\ne#{i} = base64_run(e#{i},0)\ne#{i} = base64_run(e#{i},1)\nprint(\"#{n}|\" + e#{i}[\"encoded\"] + \"|\" + str(e#{i}[\"result\"]) + \"|\" + str(base64_checksum(e#{i})))\n"
 source += "mut d#{i} := base64_prepare(basic_state({\"name\":\"Base64::Decode\",\"size\":#{n}}))\nd#{i} = base64_run(d#{i},0)\nd#{i} = base64_run(d#{i},1)\nassert_equal(d#{i}[\"decoded\"],utf8_encode(repeat(\"a\",#{n})))\nassert_equal(d#{i}[\"result\"],#{2*n})\n"
end
raise 'source size' if source.bytesize>1048576
File.write(root+'/contracts.kujo',source)
hashes[root+'/contracts.kujo']=Digest::SHA256.file(root+'/contracts.kujo').hexdigest
checksum=->(s){s.bytes.reduce(5381){|h,b|(h*33+b)&0xffffffff}}
preview=->(s){s.bytesize>4 ? s.byteslice(0,4)+'...' : s}
rows=[]
%w[vm interpreter jit].each do |mode|
 argv=[binary,'run','--scheduler-timeout-ms','60000'];argv<<'--'+mode unless mode=='vm';argv<<root+'/contracts.kujo'
 out,err,status=Open3.capture3(*argv);File.write(root+'/'+mode+'.out',out);File.write(root+'/'+mode+'.err',err)
 raise 'runtime failure '+mode unless status.success?
 lines=out.lines;raise 'coverage' unless lines.size==sizes.size
 cases=lines.zip(sizes).map do |line,n|
  size,encoded,total,actual=line.strip.split('|',-1)
  expected=Base64.strict_encode64('a'*n)
  raise 'full encoded state' unless size==n.to_s && encoded==expected && total.to_i==2*expected.bytesize
  want=checksum.call('encode '+preview.call('a'*n)+' to '+preview.call(expected)+': '+total)
  {size:n,encoded_bytes:encoded.bytesize,encoded_sha256:Digest::SHA256.hexdigest(encoded),result:total.to_i,kujo_checksum:actual.to_i,upstream_source_formula_checksum:want,checksum_matches:actual.to_i==want}
 end
 rows<<{runtime:mode,argv:argv,exit:status.exitstatus,cases:cases,stdout_sha256:Digest::SHA256.hexdigest(out),stderr_sha256:Digest::SHA256.hexdigest(err)}
end
hashes.each{|f,h|raise 'identity drift' unless Digest::SHA256.file(f).hexdigest==h}
File.write(root+'/report.json',JSON.pretty_generate({files:hashes,sizes:sizes,rows:rows,scope:'Full encoded bytes and decoded bytes/state verified in all3Kujo runtimes using unchanged port functions. Ruby standard Base64 oracle; pinned Rust/Goose source formula only, not execution. Two diagnostic repetitions at each size, not official production iteration counts. Malformed inputs and independent comparator execution remain open.'}))
mismatches=rows.sum{|row|row[:cases].count{|cell|!cell[:checksum_matches]}}
puts "36 encode/decode state cases verified; #{mismatches} encoding checksum mismatches."
exit(mismatches==0 ? 0 : 1)
