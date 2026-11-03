require 'json'
require 'digest'
require 'open3'
raise 'usage: OUTPUT [baseline|reuse] [archive COMMIT GENERATED_PORT]' unless (1..2).cover?(ARGV.size) || (ARGV.size==5 && ARGV[2]=='archive')
archived=ARGV.size==5
policy=ARGV.fetch(1, 'baseline')
raise 'unknown verification policy' unless %w[baseline reuse].include?(policy)
dir=File.expand_path(ARGV.fetch(0))
report=JSON.parse(File.read(dir+'/report.json'))
raise 'unfinished collection' unless report['status']=='verified-review-pending'
required=%w[benchmarks/langarena/probes/jit-async-worker-economics.kujo benchmarks/langarena/port.kujo benchmarks/langarena/probes/collect-jit-async-economics.rb src/interpreter/mod.rs src/interpreter/native_functions/async_ops.rs src/jit.rs src/vm.rs src/benchmarks/runtime_measurements.rs]
required+=%w[src/interpreter/tasks.rs src/interpreter/native_functions/callbacks.rs] if policy=='reuse'
raise 'source manifest incomplete' unless (required-report.fetch('files').keys).empty?
recorded_dir=dir
identity_scope='Source and executable bytes rehashed locally.'
if archived
  commit=ARGV[3]
  raise 'full archive source commit required' unless commit.match?(/\A[0-9a-f]{40}\z/)
  binary=report.fetch('files').keys.first
  raise 'archive file coverage' unless report['files'].keys.sort==(required+[binary]).sort
  raise 'archive binary receipt' unless binary.start_with?('/') && report['files'][binary].match?(/\A[0-9a-f]{64}\z/)
  required.each do |path|
    if path=='benchmarks/langarena/port.kujo'
      bytes=File.binread(ARGV[4])
    else
      bytes,err,status=Open3.capture3('git','show',"#{commit}:#{path}")
      raise "archive source missing #{path}: #{err}" unless status.success?
    end
    raise "archive input drift #{path}" unless Digest::SHA256.hexdigest(bytes)==report['files'][path]
  end
  first=report.fetch('rows').find{|r|r['runtime']=='jit'}
  index=first.fetch('argv').index('--measurements')
  raise 'archive measurement path' unless index
  recorded_dir=File.dirname(first['argv'].fetch(index+1))
  raise 'archive output directory' unless recorded_dir.start_with?('/')
  identity_scope='Pinned source and supplied generated-port bytes rehashed; executable SHA is a CI collector receipt, executable bytes unavailable for independent rehash.'
else
  report.fetch('files').each{|p,h|raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
end

labels=[]
[1,4].each{|r|labels<<"worker-direct-1-#{r}";[1,4,8,16].each{|w|labels<<"worker-parallel-#{w}-#{r}"}}
expected=labels.product(%w[vm interpreter jit loop-only])
rows=report.fetch('rows')
raise 'exact coverage' unless rows.map{|r|[r['label'],r['runtime']]}==expected
binary=report.fetch('files').keys.first
rows.each do |row|
  label=row.fetch('label')+'-'+row.fetch('runtime')
  raise 'failed outcome' unless row['passed'] && row['exit']==0 && !row['signal'] && !row['timed_out']
  raise 'wall value' unless row['process_wall_seconds'].is_a?(Numeric) && row['process_wall_seconds'].finite? && row['process_wall_seconds']>0
  raise 'raw stdout drift' unless File.read(dir+'/'+label+'.out')==row['stdout']
  raise 'raw stderr drift' unless File.read(dir+'/'+label+'.err')==row['stderr']
  raise 'worker family' unless row['family']=='worker'
  raise 'worker dimensions' unless [1,4].include?(row['repeats']) && [1,4,8,16].include?(row['workers'])
  raise 'execution' unless %w[direct parallel].include?(row['execution'])
  raise 'cell identity' unless row['label']=="worker-#{row['execution']}-#{row['workers']}-#{row['repeats']}"
  output=['async-worker-economics: OK']
  args=['benchmarks/langarena/probes/jit-async-worker-economics.kujo',row['execution'],row['workers'].to_s,row['repeats'].to_s]
  argv=[binary,'run','--scheduler-timeout-ms','60000']
  argv<<'--interpreter' if row['runtime']=='interpreter'
  argv<<'--jit' if %w[jit loop-only].include?(row['runtime'])
  argv+=['--measurements',recorded_dir+'/'+label+'.json'] unless row['runtime']=='interpreter'
  raise 'invocation' unless row['argv']==argv+args
  raise 'oracle output' unless row['stdout'].lines.map(&:strip)==output
  env=row['runtime']=='loop-only' ? {'DISABLE_FUNCTION_JIT'=>'1'} : {}
  raise 'environment' unless row['environment']==env
  if row['runtime']=='interpreter'
    raise 'interpreter metrics unsupported' if row.key?('measurement') || row['argv'].include?('--measurements')
    next
  end
  file=dir+'/'+label+'.json'
  raise 'measurement hash' unless Digest::SHA256.file(file).hexdigest==row['measurement_sha256']
  data=JSON.parse(File.read(file))
  raise 'measurement receipt' unless data==row['measurement'] && data['schema']=='kujo.runtime-measurements/v1' && data['outcome']=='success' && data['scope']=='process'
  c=data.fetch('counters')
  jobs=16*row.fetch('repeats')
  raise 'task count' unless c.fetch('tasks_started')==jobs
  if %w[jit loop-only].include?(row['runtime'])
    compiles=c.fetch('jit_compile_entries')
    if policy=='baseline'
      raise 'per-job compile count' unless compiles==jobs
    else
      raise 'reuse compile bounds' unless compiles.is_a?(Integer) && compiles.between?(1,jobs)
      if row['execution']=='direct' || row['workers']==1
        raise 'serial reuse did not eliminate per-job compilation' unless compiles==1
      end
    end
    raise 'native boundary count' unless c.fetch('jit_native_boundary_entries')==jobs
    raise 'native function count' unless c.fetch('jit_native_function_entries')==(row['runtime']=='jit' ? jobs : 0)
    if row['runtime']=='loop-only'
      raise 'loop cache' unless c.fetch('jit_cache_lookup_misses')==compiles && c.fetch('jit_cache_lookup_hits')==jobs-compiles
    end
  else
    raise 'unexpected VM compilation' unless c.fetch('jit_compile_entries')==0
  end
  %w[jit_compile_entries jit_compile_inclusive_wall_ns jit_native_function_entries].each do |key|
    value=data.fetch('counters').fetch(key)
    raise 'counter value' unless value.is_a?(Integer) && value>=0
  end
end
summary=rows.select{|r|%w[jit loop-only].include?(r['runtime'])}.map do |row|
  c=row.fetch('measurement').fetch('counters')
  row.slice('label','runtime').merge('compile_entries'=>c['jit_compile_entries'],'compile_inclusive_wall_ns'=>c['jit_compile_inclusive_wall_ns'],'native_function_entries'=>c['jit_native_function_entries'])
end
File.write(dir+(archived ? '/archived-verified.json' : '/verified.json'),JSON.pretty_generate({status:archived ? 'archived-counters-verified' : 'passed',input_identity_scope:identity_scope,verification_policy:policy,outcomes:40,report_sha256:Digest::SHA256.file(dir+'/report.json').hexdigest,scope:report['scope'],summary:summary})+"\n")
puts archived ? '40 archived diagnostic cells and counters verified; executable identity remains CI-receipt evidence.' : '40 exact diagnostic cells, outputs, measurement receipts and input identities verified.'
