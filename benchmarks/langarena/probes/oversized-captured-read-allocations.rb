# Oversized captured-read regression gate; exercised helpers and loops are Kujo.
# Probe and binary must come from the same intended normal release build.
# Counts are allocation traffic, not elapsed speed or retained heap.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR' unless ARGV.size==3
probe,binary,d=ARGV.map{|x|File.expand_path(x)}
abort 'Output already exists' if File.exist?(d)
[probe,binary].each{|path|abort 'Expected executable probe and runtime' unless File.file?(path)&&File.executable?(path)}
FileUtils.mkdir_p(d)
report_path=d+'/report.json'
r={runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,probe_sha256:Digest::SHA256.file(probe).hexdigest,binary_sha256:Digest::SHA256.file(binary).hexdigest,rows:[],semantics:[],status:'running'}
save=->{File.write(report_path,JSON.pretty_generate(r)+"\n")};save.call
begin
 %w[mut let].each do |kind|
  [[0,128],[0,2048],[128,128],[8192,128]].each do |bindings,width|
   dir=d+"/captured-read-fixtures/#{kind}-#{bindings}-#{width}";FileUtils.mkdir_p(dir)
   mod="mut state := range(7, #{width+7})\n"+(0...bindings).map{|n|"#{kind} extra_#{n} := #{n}\n"}.join+"export func first_captured() { return state[0] }\n"
   File.write(dir+'/read_state.kujo',mod)
   [64,128].each do |calls|
    source="from read_state import first_captured\nmut i := 0\nmut total := 0\nwhile i < #{calls} { total += first_captured() i += 1 }\nassert_equal(total, #{calls*7})\nassert_equal(i, #{calls})\nprint(\"CAPTURED READ PASS\")\n"
    path=dir+"/calls-#{calls}.kujo"
    File.write(path,source)
    meta={kind:kind,bindings:bindings,width:width,calls:calls,source:source,module_source:mod,source_sha256:Digest::SHA256.file(path).hexdigest,module_sha256:Digest::SHA256.file(dir+'/read_state.kujo').hexdigest}
    %w[vm jit].each do |mode|
     out,err,status=Open3.capture3(probe,mode,path);raise out+err unless status.success? && out=="CAPTURED READ PASS\n"
     counts=err.match(/requests=(\d+) requested_bytes=(\d+)/);raise err unless counts
     r[:rows]<<meta.merge(mode:mode,requests:counts[1].to_i,bytes:counts[2].to_i);save.call
    end
    %w[vm interpreter jit].each do |mode|
     args=[binary,'run',path];args<<'--'+mode unless mode=='vm'
     out,err,status=Open3.capture3(*args);raise out+err unless status.success? && out=="CAPTURED READ PASS\n"
     r[:semantics]<<meta.reject{|k,v|[:source,:module_source].include?(k)}.merge(mode:mode,status:status.exitstatus);save.call
    end
   end
  end
 end
 raise 'missing cases' unless r[:rows].size==32 && r[:semantics].size==48
 r[:deltas]=r[:rows].group_by{|x|[x[:mode],x[:kind],x[:bindings],x[:width]]}.map do |key,rs|
  a,b=rs.sort_by{|x|x[:calls]};{mode:key[0],kind:key[1],bindings:key[2],width:key[3],requests_per_call:(b[:requests]-a[:requests])/64.0,bytes_per_call:(b[:bytes]-a[:bytes])/64.0}
 end
 r[:flat_setup]=r[:deltas].group_by{|x|[x[:mode],x[:kind]]}.all?{|key,rs|rs.size==4&&rs.map{|x|[x[:requests_per_call],x[:bytes_per_call]]}.uniq.size==1}
 raise 'Captured read setup scales with unrelated bindings or collection width' unless r[:flat_setup]
 r[:status]='passed';save.call
 puts "32 allocation and48 semantic controls passed; captured read setup is flat"
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
