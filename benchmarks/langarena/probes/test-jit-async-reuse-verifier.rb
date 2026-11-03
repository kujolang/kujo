# Synthetic parser controls only. These fixtures are not benchmark measurements.
require 'tmpdir'
require 'fileutils'
require 'json'
require 'digest'
require 'open3'
verifier=File.expand_path('verify-jit-async-economics.rb',__dir__)
results=[]
Dir.mktmpdir('synthetic-async-reuse-') do |dir|
  binary=dir+'/synthetic-binary'; File.write(binary,'not an executable')
  %w[baseline reuse duplicate failed serial-regression native-loss].each do |control|
    rows=[]
    [1,4].each do |repeats|
      [['direct',1],['parallel',1],['parallel',4],['parallel',8],['parallel',16]].each do |execution,workers|
        %w[vm interpreter jit loop-only].each do |runtime|
          label="worker-#{execution}-#{workers}-#{repeats}"; file=dir+'/'+label+'-'+runtime
          argv=[binary,'run','--scheduler-timeout-ms','60000']
          argv<<'--interpreter' if runtime=='interpreter'
          argv<<'--jit' if %w[jit loop-only].include?(runtime)
          argv+=['--measurements',file+'.json'] unless runtime=='interpreter'
          argv+=['benchmarks/langarena/probes/jit-async-worker-economics.kujo',execution,workers.to_s,repeats.to_s]
          row={label:label,family:'worker',workers:workers,repeats:repeats,execution:execution,runtime:runtime,argv:argv,environment:runtime=='loop-only' ? {'DISABLE_FUNCTION_JIT'=>'1'} : {},exit:0,signal:nil,timed_out:false,passed:true,process_wall_seconds:1,stdout:"async-worker-economics: OK\n",stderr:''}
          File.write(file+'.out',row[:stdout]);File.write(file+'.err','')
          unless runtime=='interpreter'
            jobs=16*repeats; native=%w[jit loop-only].include?(runtime)
            compiles=native ? (control=='baseline' || workers>1 ? jobs : 1) : 0
            compiles=jobs if control=='serial-regression' && native
            counters={'tasks_started'=>jobs,'jit_compile_entries'=>compiles,'jit_compile_inclusive_wall_ns'=>1,'jit_native_boundary_entries'=>native ? jobs : 0,'jit_native_function_entries'=>runtime=='jit' ? jobs : 0,'jit_cache_lookup_misses'=>compiles,'jit_cache_lookup_hits'=>native ? jobs-compiles : 0}
            counters['jit_native_boundary_entries']=0 if control=='native-loss' && native
            data={schema:'kujo.runtime-measurements/v1',outcome:'success',scope:'process',counters:counters}
            File.write(file+'.json',JSON.generate(data));row[:measurement]=data;row[:measurement_sha256]=Digest::SHA256.file(file+'.json').hexdigest
          end
          rows<<row
        end
      end
    end
    rows[-1]=rows[0].dup if control=='duplicate'
    rows[0][:exit]=4 if control=='failed'
    paths=[binary]+%w[benchmarks/langarena/probes/jit-async-worker-economics.kujo benchmarks/langarena/port.kujo benchmarks/langarena/probes/collect-jit-async-economics.rb src/interpreter/mod.rs src/interpreter/native_functions/async_ops.rs src/jit.rs src/vm.rs src/benchmarks/runtime_measurements.rs src/interpreter/tasks.rs src/interpreter/native_functions/callbacks.rs]
    paths.drop(1).each do |path|
      FileUtils.mkdir_p(File.dirname(dir+'/'+path))
      File.write(dir+'/'+path,'synthetic source fixture; not benchmark evidence')
    end
    report={status:'verified-review-pending',files:paths.to_h{|p|[p,Digest::SHA256.file(p==binary ? p : dir+'/'+p).hexdigest]},rows:rows,scope:'Synthetic verifier test; not benchmark evidence'}
    File.write(dir+'/report.json',JSON.generate(report))
    policy=control=='baseline' ? 'baseline' : 'reuse'
    out,err,status=Open3.capture3('ruby',verifier,dir,policy,chdir:dir)
    expected=%w[baseline reuse].include?(control)
    raise "#{control}: #{out} #{err}" unless status.success? == expected
    results<<{control:control,accepted:status.success?}
  end
end
puts JSON.pretty_generate({synthetic:true,controls:results})
