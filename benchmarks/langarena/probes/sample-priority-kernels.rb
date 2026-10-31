# macOS diagnostic sampling after warmup, never an elapsed-speed comparison.
require 'json';require 'digest';require 'open3';require 'fileutils'
binary=File.realpath(ARGV.fetch(0));root=File.expand_path(ARGV.fetch(1));repo=File.realpath('.')
raise 'new output directory required' if File.exist?(root);FileUtils.mkdir_p(root)
sha=->(p){Digest::SHA256.file(p).hexdigest};config=repo+'/benchmarks/langarena/test.json';port=repo+'/benchmarks/langarena/port.kujo'
cases=JSON.parse(File.read(config));report={status:'running',pid:Process.pid,binary:binary,binary_sha256:sha.call(binary),sources:[config,port,File.expand_path(__FILE__)].to_h{|p|[p,sha.call(p)]},rows:[],limits:'Three-second macOS stack samples begin at warmup-done. They may be partial if execution ends early, and cannot establish clean-host speed or precise generated-native time coverage.'}
write=->{File.write(root+'/report.json.tmp',JSON.pretty_generate(report)+"\n");File.rename(root+'/report.json.tmp',root+'/report.json')};write.call
begin
 %w[Sort::Quick Sort::Merge Etc::NeuralNet].each do |name|
  %w[vm jit].each do |mode|
   cfg=cases.find{|c|c['name']==name};raise unless cfg
   stem=mode+'-'+name.gsub('::','-');argv=[binary,'run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit';argv += [port,config,name,'--profile']
   stdout='';stderr='';sample=nil;exitcode=nil
   Open3.popen3(*argv) do |input,out,err,child|
    input.close;errors=Thread.new{err.read};sampler=nil
    out.each_line do |line|
     stdout<<line
     if line.start_with?('@profile warmup-done ') && sampler.nil?
      sampler=Thread.new do
       command=['/usr/bin/sample',child.pid.to_s,'3','2','-mayDie','-file',root+'/'+stem+'.sample.txt']
       log,sampler_errors,sampler_status=Open3.capture3(*command)
       {argv:command,exit:sampler_status.exitstatus,stdout:log,stderr:sampler_errors,trigger_line:line.strip}
      end
     end
    end
    exitcode=child.value.exitstatus;stderr=errors.value;sample=sampler&.value
   end
   row={name:name,mode:mode,argv:argv,exit:exitcode,stdout:stdout,stderr:stderr,sample:sample}
   path=root+'/'+stem+'.sample.txt';row[:sample_file]=File.basename(path) if File.file?(path);row[:sample_sha256]=sha.call(path) if File.file?(path)
   report[:rows]<<row;write.call
   lines=stdout.lines.map(&:strip);raise 'checksum' unless exitcode==0&&lines.include?("#{name}: OK #{cfg['checksum']}")&&lines.include?('Summary: 1, 1, 0')
   raise 'missing warmup trigger' unless sample
  end
 end
 report[:sources].each{|p,h|raise 'source drift' unless sha.call(p)==h};raise 'binary drift' unless sha.call(binary)==report[:binary_sha256]
 report[:status]='measured';write.call
rescue=>e;report[:status]='failed';report[:error]=e.message;write.call;raise
end
