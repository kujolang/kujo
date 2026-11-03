require 'json'
require 'digest'
dir=File.expand_path(ARGV.fetch(0))
report=JSON.parse(File.read(dir+'/report.json'))
raise 'unfinished collection' unless report['status']=='verified-review-pending'
report.fetch('files').each{|p,h|raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
labels=[]
%w[straight loop].each{|f|(f=='straight' ? [1,16,128] : [0,8,2000]).each{|n|[1,99,100,101,1000].each{|c| labels << "#{f}-#{n}-#{c}"}}}
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
  family=row.fetch('family');size=row.fetch('size');calls=row.fetch('calls')
  raise 'cell identity' unless row['label']=="#{family}-#{size}-#{calls}"
  body=if family=='straight'
    "func kernel(n) {\nmut x := n\n" + "x += 1\n"*size + "return x\n}\n"
  else
    "func kernel(n) {\nmut x := 0\nmut i := 0\nwhile i < n { x += i i += 1 }\nreturn x\n}\n"
  end
  argument=family=='straight' ? 7 : size
  result=family=='straight' ? 7+size : (0...size).sum
  program=body+"mut total := 0\n"+"total += kernel(#{argument})\n"*calls+"assert_equal(total, #{result*calls})\nprint(\"admission: OK #{result*calls}\")\n"
  path=dir+'/'+row['label']+'.kujo'
  raise 'program' unless File.read(path)==program
  output=["admission: OK #{result*calls}"]
  args=[path]
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
  c=data.fetch('counters')
  raise 'unexpected task' unless c.fetch('tasks_started')==0
  if row['runtime']=='vm'
    raise 'unexpected VM compilation' unless c.fetch('jit_compile_entries')==0
  end
  if row['runtime']=='jit'
    compiled=family=='loop' || calls>=100
    native=family=='loop' ? calls : [calls-99,0].max
    raise 'function admission' unless c.fetch('jit_compile_entries')==(compiled ? 1 : 0) && c.fetch('jit_native_function_entries')==native && c.fetch('jit_native_boundary_entries')==native
  elsif row['runtime']=='loop-only'
    native=family=='straight' || size==0 ? 0 : (size==8 ? [calls-12,0].max : calls)
    raise 'loop admission' unless c.fetch('jit_compile_entries')==(native>0 ? 1 : 0) && c.fetch('jit_native_function_entries')==0 && c.fetch('jit_native_boundary_entries')==native
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
File.write(dir+'/verified.json',JSON.pretty_generate({status:'passed',outcomes:120,report_sha256:Digest::SHA256.file(dir+'/report.json').hexdigest,scope:report['scope'],summary:summary})+"\n")
puts '120 exact diagnostic cells, outputs, measurement receipts and input identities verified.'
