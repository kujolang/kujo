# Host-side process deadlines, artifact identity and telemetry only. Kernels are Kujo.
require 'json'
require 'digest'
require 'fileutils'
$stdout.sync=true
root=File.expand_path(ARGV.fetch(0)); stage=ARGV.fetch(1)
FileUtils.mkdir_p(root+'/runs')
report_path=root+'/'+stage+'.json'
abort 'Refusing to overwrite an existing report' if File.exist?(report_path)
report={stage:stage,started_at:Time.now.utc.to_s,scope:'Local diagnostic; no clean-host ranking',rows:[],telemetry:[]}
report[:hashes]={}
["#{root}/candidate-kujo","#{root}/original-kujo","#{root}/isolation-probe",'benchmarks/langarena/port.kujo','benchmarks/langarena/test.json'].each{|p|report[:hashes][p]=Digest::SHA256.file(p).hexdigest if File.file?(p)}
save=-> { File.write(report_path,JSON.pretty_generate(report)+"\n") }
stop=false
observer=Thread.new do
 until stop
  report[:telemetry] << {at:Time.now.utc.to_s,load:`uptime`,memory:`vm_stat`,thermal:`pmset -g therm`}
  50.times{break if stop;sleep 0.1}
 end
end
run=->(label,argv,meta,expected) do
 stem=root+'/runs/'+label
 abort 'Output exists' if File.exist?(stem+'.stdout')
 start=Process.clock_gettime(Process::CLOCK_MONOTONIC)
 pid=Process.spawn({'KUJO_ALLOCATION_PHASE'=>'0','DEBUG_JIT_CALL'=>nil,'DEBUG_VM'=>nil},*argv,out:stem+'.stdout',err:stem+'.stderr',pgroup:true)
 timeout=false
 waiter=Thread.new{Process.waitpid2(pid)[1]}
 unless waiter.join(180)
  timeout=true
  begin Process.kill('TERM',-pid);rescue Errno::ESRCH;end
  unless waiter.join(1)
   begin Process.kill('KILL',-pid);rescue Errno::ESRCH;end
  end
 end
 status=waiter.value
 elapsed=Process.clock_gettime(Process::CLOCK_MONOTONIC)-start
 out=File.read(stem+'.stdout');err=File.read(stem+'.stderr')
 ok=status.success? && !timeout && expected.all?{|line|out.lines.map(&:strip).include?(line)}
 row=meta.merge(label:label,argv:argv,elapsed_seconds:elapsed,exitcode:status.exitstatus,status:timeout ? 'timeout' : ok ? 'passed' : 'failed',stdout:out,stderr:err)
 report[:rows] << row;save.call
 puts "#{row[:status]} #{label}"
 abort "Invalid diagnostic #{label}" unless ok
end
begin
 case stage
 when 'neural'
  7.times do |round|
   %w[vm jit].each do |mode|
    variants=round.even? ? %w[original candidate] : %w[candidate original]
    variants.each do |variant|
     argv=[root+'/'+variant+'-kujo','run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit'
     argv+=['benchmarks/langarena/port.kujo','benchmarks/langarena/test.json','Etc::NeuralNet','--profile']
     run.call("neural-#{round}-#{mode}-#{variant}",argv,{round:round+1,runtime:mode,variant:variant},['Etc::NeuralNet: OK 1586674334','Summary: 1, 1, 0'])
    end
   end
  end
 when 'priority_calls'
  configurations=JSON.parse(File.read('benchmarks/langarena/test.json'))
  7.times do |round|
   ['Sort::Quick','Sort::Merge','Etc::NeuralNet'].each do |name|
    expected=configurations.find{|c|c['name']==name}.fetch('checksum')
    %w[vm jit].each do |mode|
     (round.even? ? %w[original candidate] : %w[candidate original]).each do |variant|
      argv=[root+'/'+variant+'-kujo','run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit'
      argv+=['benchmarks/langarena/port.kujo','benchmarks/langarena/test.json',name,'--profile']
      label="priority-#{name.gsub(/[^a-zA-Z0-9]/,'-')}-#{round}-#{mode}-#{variant}"
      run.call(label,argv,{round:round+1,benchmark:name,runtime:mode,variant:variant},["#{name}: OK #{expected}",'Summary: 1, 1, 0'])
     end
    end
   end
  end
 when 'neural_resources'
  %w[vm jit].each do |mode|
   %w[original candidate].each do |variant|
    argv=['/usr/bin/time','-l',root+'/'+variant+'-kujo','run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit'
    argv+=['benchmarks/langarena/port.kujo','benchmarks/langarena/test.json','Etc::NeuralNet','--profile']
    run.call("resources-#{mode}-#{variant}",argv,{runtime:mode,variant:variant,observation:'Separate resource-accounting run, excluded from paired timing statistics'},['Etc::NeuralNet: OK 1586674334','Summary: 1, 1, 0'])
   end
  end
 when 'ownership','phase','ownership_controls'
  fixture_file=stage=='ownership_controls' ? '/ownership-controls-fixtures.json' : '/fixtures.json'
  fixtures=JSON.parse(File.read(root+fixture_file)).select{|r|r['kind']==stage}
  ownership=stage.start_with?('ownership')
  fixtures.each_with_index do |f,i|
   modes=ownership ? %w[vm interpreter jit] : %w[vm jit]
   modes.each do |mode|
    expected=ownership ? ["ISOLATION PASS #{f['shape']} #{f['owner']} #{f['size']}"] : [f['name']+': OK '+JSON.parse(File.read('benchmarks/langarena/test.json')).find{|c|c['name']==f['name']}['checksum'].to_s,'Summary: 1, 1, 0']
    meta=f.merge('runtime'=>mode,'source_sha256'=>Digest::SHA256.file(f['path']).hexdigest)
    if mode=='interpreter'
     run.call("#{stage}-#{i}-#{mode}",[root+'/candidate-kujo','run','--scheduler-no-timeout','--interpreter',f['path']],meta,expected)
    else
     run.call("#{stage}-#{i}-#{mode}",[root+'/isolation-probe',mode,f['path'],'count'],meta,expected)
    end
   end
  end
 when 'micro'
  run.call('micro',[root+'/isolation-probe','micro'],{},[])
 when 'break_even','break_even_confirmation'
  rounds=stage=='break_even' ? (0...3) : (3...7)
  rounds.each do |round|
   [32,512,8192,131072].each do |iterations|
    modes=round.even? ? %w[vm jit] : %w[jit vm]
    config=root+"/base64-#{iterations}.json"
    cfg=JSON.parse(File.read(config)).first
    modes.each do |mode|
     argv=[root+'/candidate-kujo','run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit'
     argv+=['benchmarks/langarena/port.kujo',config,'Base64::Encode','--profile']
     run.call("break-even-#{round}-#{iterations}-#{mode}",argv,{round:round+1,runtime:mode,iterations:iterations,config:cfg,config_sha256:Digest::SHA256.file(config).hexdigest},["Base64::Encode: OK #{cfg['checksum']}",'Summary: 1, 1, 0'])
    end
   end
  end
 else abort 'Unknown stage'
 end
ensure
 stop=true;observer.join;report[:finished_at]=Time.now.utc.to_s;save.call
end
