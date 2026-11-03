require 'json'
require 'digest'
dir=File.expand_path(ARGV.fetch(0))
report=JSON.parse(File.read(dir+'/report.json'))
raise 'unfinished collection' unless report['status']=='verified-review-pending'
report.fetch('files').each{|p,h|raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
labels=%w[Obj Arena].product([1,3,30]).map{|t,n|"tree-#{t}-#{n}"}
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
  if row['family']=='tree'
    name='Binarytrees::'+row['label'].split('-')[1]
    raise 'tree dimensions' unless row['iterations']==Integer(row['label'].split('-')[2])
    output=["#{name}: OK #{31*(row.fetch('iterations')+1)}",'Summary: 1, 1, 0']
    cfg=dir+'/'+row['label']+'.json'
    raise 'tree config' unless JSON.parse(File.read(cfg))==[{'name'=>name,'depth'=>4,'iterations'=>row['iterations'],'warmup_iterations'=>1,'checksum'=>31*(row['iterations']+1)}]
    args=['benchmarks/langarena/port.kujo',cfg,name]
  else
    raise 'worker dimensions' unless [1,4].include?(row['repeats']) && [1,4,8,16].include?(row['workers'])
    output=['worker-economics: OK']
    args=['benchmarks/langarena/probes/jit-worker-economics.kujo',row['execution'],row['workers'].to_s,row['repeats'].to_s]
  end
  argv=[binary,'run','--scheduler-timeout-ms','60000']
  argv<<'--interpreter' if row['runtime']=='interpreter'
  argv<<'--jit' if %w[jit loop-only].include?(row['runtime'])
  argv+=['--measurements',dir+'/'+label+'.json'] unless row['runtime']=='interpreter'
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
  %w[jit_compile_entries jit_compile_inclusive_wall_ns jit_native_function_entries].each do |key|
    value=data.fetch('counters').fetch(key)
    raise 'counter value' unless value.is_a?(Integer) && value>=0
  end
end
summary=rows.select{|r|%w[jit loop-only].include?(r['runtime'])}.map do |row|
  c=row.fetch('measurement').fetch('counters')
  row.slice('label','runtime').merge('compile_entries'=>c['jit_compile_entries'],'compile_inclusive_wall_ns'=>c['jit_compile_inclusive_wall_ns'],'native_function_entries'=>c['jit_native_function_entries'])
end
File.write(dir+'/verified.json',JSON.pretty_generate({status:'passed',outcomes:64,report_sha256:Digest::SHA256.file(dir+'/report.json').hexdigest,scope:report['scope'],summary:summary})+"\n")
puts '64 exact diagnostic cells, outputs, measurement receipts and input identities verified.'
