# Local diagnostic collector; counters are not uninstrumented speed samples.
require 'json'
require 'digest'
require 'fileutils'
require 'time'
abort 'usage: collect-jit-admission.rb FROZEN_BINARY NEW_OUTPUT' unless ARGV.size == 2
binary, dir = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(dir)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
source = 'src/compiler.rs'
port = 'benchmarks/langarena/port.kujo'
files = [binary, source, port, __FILE__, 'src/interpreter/mod.rs', 'src/interpreter/native_functions/async_ops.rs', 'src/jit.rs', 'src/vm.rs', 'src/benchmarks/runtime_measurements.rs'].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
verify = -> { files.each { |p,h| raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest == h } }
FileUtils.mkdir_p(dir)
report = {status:'running',pid:Process.pid,started_at:Time.now.utc.iso8601,files:files,rows:[],scope:'One instrumented observation per cell. Independent straight-line call sites avoid a compiled caller loop. Vary static arithmetic body size or dynamic loop trip count and calls around threshold100 plus1000calls. Includes setup and checksum; no paired speed claim. Compiler wall is inclusive, not self CPU. Interpreter has no measurement flag.'}
save = -> { File.write(dir+'/report.json.tmp', JSON.pretty_generate(report)+"\n"); File.rename(dir+'/report.json.tmp',dir+'/report.json') }
save.call
cells=[]
%w[straight loop].each do |family|
  sizes=family=='straight' ? [1,16,128] : [0,8,2000]
  sizes.each do |size|
    [1,99,100,101,1000].each do |calls|
      label="#{family}-#{size}-#{calls}"
      body=if family=='straight'
        "func kernel(n) {\nmut x := n\n" + "x += 1\n"*size + "return x\n}\n"
      else
        "func kernel(n) {\nmut x := 0\nmut i := 0\nwhile i < n { x += i i += 1 }\nreturn x\n}\n"
      end
      argument=family=='straight' ? 7 : size
      result=family=='straight' ? 7+size : size*(size-1)/2
      program=body+"mut total := 0\n"+"total += kernel(#{argument})\n"*calls+"assert_equal(total, #{result*calls})\nprint(\"admission: OK #{result*calls}\")\n"
      path=dir+'/'+label+'.kujo'
      File.write(path,program)
      files[path]=Digest::SHA256.file(path).hexdigest
      cells << {label:label,family:family,size:size,calls:calls,argv:[path],expected:["admission: OK #{result*calls}"]}
    end
  end
end

begin
  cells.each do |cell|
    %w[vm interpreter jit loop-only].each do |runtime|
      verify.call
      label=cell[:label]+'-'+runtime
      argv=[binary,'run','--scheduler-timeout-ms','60000']
      argv << '--interpreter' if runtime=='interpreter'
      argv << '--jit' if %w[jit loop-only].include?(runtime)
      measurement=dir+'/'+label+'.json'
      argv += ['--measurements',measurement] unless runtime=='interpreter'
      argv += cell[:argv]
      env=runtime=='loop-only' ? {'DISABLE_FUNCTION_JIT'=>'1'} : {}
      out=dir+'/'+label+'.out'; err=dir+'/'+label+'.err'
      started=Process.clock_gettime(Process::CLOCK_MONOTONIC)
      pid=Process.spawn(env,*argv,out:out,err:err,pgroup:true)
      report[:child_pid]=pid;report[:current]=label;save.call
      status=nil;timed_out=false
      loop do
        if (result=Process.wait2(pid,Process::WNOHANG)); status=result.last;break;end
        if Process.clock_gettime(Process::CLOCK_MONOTONIC)-started>120
          timed_out=true
          Process.kill('TERM',-pid) rescue nil
          sleep 0.2
          Process.kill('KILL',-pid) rescue nil
          status=Process.wait2(pid).last;break
        end
        sleep 0.02
      end
      report.delete(:child_pid)
      stdout=File.read(out);stderr=File.read(err)
      passed=status.success? && !timed_out && stdout.lines.map(&:strip)==cell[:expected]
      row=cell.reject{|k,_|[:argv,:expected].include?(k)}.merge(runtime:runtime,argv:argv,environment:env,exit:status.exitstatus,signal:status.termsig,timed_out:timed_out,passed:passed,process_wall_seconds:Process.clock_gettime(Process::CLOCK_MONOTONIC)-started,stdout:stdout,stderr:stderr)
      report[:rows]<<row;save.call
      raise "failed #{label}" unless passed
      unless runtime=='interpreter'
        data=JSON.parse(File.read(measurement))
        raise 'measurement contract' unless data['schema']=='kujo.runtime-measurements/v1' && data['outcome']=='success' && data['scope']=='process'
        row[:measurement]=data;row[:measurement_sha256]=Digest::SHA256.file(measurement).hexdigest
      end
      verify.call;save.call
    end
  end
  raise 'coverage' unless report[:rows].size==120
  report[:status]='verified-review-pending'
rescue => error
  report[:status]='failed';report[:error]=error.message
  raise
ensure
  report[:finished_at]=Time.now.utc.iso8601;save.call
end
puts '120 admission diagnostic outcomes verified; review counters before proposing an admission policy.'
