# Retained-local capture characterization; allocation traffic is not elapsed speed.
# Separate closures retain independent snapshots; aliases share their cells.
require 'json';require 'digest';require 'fileutils';require 'open3'
abort 'Expected MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR [BASELINE_PROBE]' unless [3,4].include?(ARGV.size)
probe,binary,d=ARGV.first(3).map { |p| File.expand_path(p) }
baseline=ARGV[3] && File.expand_path(ARGV[3])
raise 'Output already exists' if File.exist?(d)
[probe,binary,baseline].compact.each { |p| raise 'Expected executable artifacts' unless File.file?(p) && File.executable?(p) }
FileUtils.mkdir_p(d)
r={runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,status:'running',probe_sha256:Digest::SHA256.file(probe).hexdigest,binary_sha256:Digest::SHA256.file(binary).hexdigest,rows:[],semantics:[],fixtures:[]}
if baseline
 r[:baseline_probe_sha256]=Digest::SHA256.file(baseline).hexdigest
 r[:baseline_rows]=[]
end
save=->{File.write(d+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 [0,128,2048,8192].each do |bindings|
  dir=d+"/bindings-#{bindings}";FileUtils.mkdir_p(dir)
  retained="if flag { mut check := 0\n"+(0...bindings).map{|i|"check += extra_#{i}\n"}.join+"assert_equal(check, #{bindings*(bindings-1)/2}) }\n"
  mod="export func factory() {\nmut counter := 7\n"+(0...bindings).map{|i|"mut extra_#{i} := #{i}\n"}.join+"func read(flag) { #{retained} return counter }\nfunc write(flag) { #{retained} counter += 1 return counter }\nreturn [read, write]\n}\n"
  File.write(dir+'/local_state.kujo',mod)
  %w[read write].each do |op|
   [1,16,32].each do |calls|
    expected=calls*7+(op=='write' ? calls*(calls+1)/2 : 0);final=7+(op=='write' ? calls : 0)
    source="from local_state import factory\nlet helpers := factory()\nlet chosen := helpers[#{op=='read' ? 0 : 1}]\nmut i := 0\nmut total := 0\nwhile i < #{calls} { total += chosen(false) i += 1 }\nassert_equal(total, #{expected})\nassert_equal(helpers[0](false), 7)\nlet alias := chosen\nassert_equal(alias(true), #{final+(op=='write' ? 1 : 0)})\nprint(\"LOCAL CAPTURE PASS\")\n"
    path=dir+"/#{op}-#{calls}.kujo";File.write(path,source)
    meta={bindings:bindings,operation:op,calls:calls,source_sha256:Digest::SHA256.hexdigest(source),module_sha256:Digest::SHA256.hexdigest(mod)}
    r[:fixtures]<<meta.merge(source:source,module_source:mod)
    %w[vm jit].each do |mode|
     out,err,status=Open3.capture3(probe,mode,path);raise out+err unless status.success?&&out=="LOCAL CAPTURE PASS\n"
     c=err.match(/requests=(\d+) requested_bytes=(\d+)/);raise err unless c
     r[:rows]<<meta.merge(mode:mode,requests:c[1].to_i,bytes:c[2].to_i);save.call
     if baseline
      before_out,before_err,before_status=Open3.capture3(baseline,mode,path)
      raise before_out+before_err unless before_status.success?&&before_out=="LOCAL CAPTURE PASS\n"
      before=before_err.match(/requests=(\d+) requested_bytes=(\d+)/);raise before_err unless before
      r[:baseline_rows]<<meta.merge(mode:mode,requests:before[1].to_i,bytes:before[2].to_i);save.call
     end
    end
    %w[vm interpreter jit].each do |mode|
     args=[binary,'run',path];args<<'--'+mode unless mode=='vm'
     out,err,status=Open3.capture3(*args);raise out+err unless status.success?&&out=="LOCAL CAPTURE PASS\n"
     r[:semantics]<<meta.merge(mode:mode,exit:status.exitstatus);save.call
    end
   end
  end
 end
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:bindings,:operation,:mode)}.map do |key,rows|
  a,b=rows.select{|x|[16,32].include?(x[:calls])}.sort_by{|x|x[:calls]};{bindings:key[0],operation:key[1],mode:key[2],requests_per_call:(b[:requests]-a[:requests])/16.0,bytes_per_call:(b[:bytes]-a[:bytes])/16.0}
 end
 raise 'Missing controls' unless r[:rows].size==48&&r[:semantics].size==72
 if baseline
  raise 'Missing baseline controls' unless r[:baseline_rows].size==48
  r[:paired_totals]=r[:rows].map do |row|
   before=r[:baseline_rows].find{|x|x.values_at(:bindings,:operation,:mode,:calls)==row.values_at(:bindings,:operation,:mode,:calls)}
   row.merge(requests_delta:row[:requests]-before[:requests],bytes_delta:row[:bytes]-before[:bytes])
  end
  r[:paired_per_call]=r[:deltas].map do |row|
   before=r[:baseline_rows].select{|x|x.values_at(:bindings,:operation,:mode)==row.values_at(:bindings,:operation,:mode)&&[16,32].include?(x[:calls])}.sort_by{|x|x[:calls]}
   row.merge(requests_delta:row[:requests_per_call]-(before[1][:requests]-before[0][:requests])/16.0,bytes_delta:row[:bytes_per_call]-(before[1][:bytes]-before[0][:bytes])/16.0)
  end
  r[:no_allocation_regression]=(r[:paired_totals]+r[:paired_per_call]).all?{|x|x[:requests_delta]<=0&&x[:bytes_delta]<=0}
  r[:oversized_reduced]=r[:paired_per_call].select{|x|x[:bindings]==8192}.all?{|x|x[:requests_delta]<0&&x[:bytes_delta]<0}
  raise 'Local capture allocation regression' unless r[:no_allocation_regression]
 end
 r[:status]=baseline ? 'passed' : 'characterized';save.call;puts JSON.pretty_generate(r[:deltas])
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
