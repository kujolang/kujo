# Local diagnostic collector; counters are not uninstrumented speed samples.
require 'json'
require 'digest'
require 'fileutils'
require 'time'
abort 'usage: collect-jit-async-economics.rb FROZEN_BINARY NEW_OUTPUT' unless ARGV.size == 2
binary, dir = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(dir)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
source = 'benchmarks/langarena/probes/jit-async-worker-economics.kujo'
port = 'benchmarks/langarena/port.kujo'
files = [binary, source, port, __FILE__, 'src/interpreter/mod.rs', 'src/interpreter/tasks.rs', 'src/interpreter/native_functions/callbacks.rs', 'src/interpreter/native_functions/async_ops.rs', 'src/jit.rs', 'src/vm.rs', 'src/benchmarks/runtime_measurements.rs'].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
verify = -> { files.each { |p,h| raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest == h } }
FileUtils.mkdir_p(dir)
report = {status:'running',pid:Process.pid,started_at:Time.now.utc.iso8601,files:files,rows:[],scope:'One instrumented observation per cell; Async wrapper calls a synchronous helper; fixed 16 equal worker jobs per repetition. Direct controls await each wrapper; parallel controls use the async mapper route. Counters include setup/checksum and compile attempts. Inclusive compile wall spans can overlap across workers, exclude metadata retention before their entry, and are not CPU time or a fraction of process time. Interpreter has no measurement flag.'}
save = -> { File.write(dir+'/report.json.tmp', JSON.pretty_generate(report)+"\n"); File.rename(dir+'/report.json.tmp',dir+'/report.json') }
save.call
cells=[]
[1,4].each do |repeats|
  [['direct',1]] .concat([1,4,8,16].map { |w| ['parallel',w] }).each do |mode,workers|
    cells << {label:"worker-#{mode}-#{workers}-#{repeats}",family:'worker',workers:workers,repeats:repeats,execution:mode,argv:[source,mode,workers.to_s,repeats.to_s],expected:['async-worker-economics: OK']}
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
  raise 'coverage' unless report[:rows].size==40
  report[:status]='verified-review-pending'
rescue => error
  report[:status]='failed';report[:error]=error.message
  raise
ensure
  report[:finished_at]=Time.now.utc.iso8601;save.call
end
puts '40 async diagnostic outcomes verified; review counters before proposing an admission policy.'
