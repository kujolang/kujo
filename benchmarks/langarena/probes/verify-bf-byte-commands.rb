require 'json'
require 'digest'
raise 'usage: STATE_DIRECTORY TIMING_DIRECTORY' unless ARGV.size==2
state,timing=ARGV.map { |p| File.expand_path(p) }
manifest=JSON.parse(File.read(state+'/manifest.json'))
manifest.fetch('files').each { |p,h| raise 'state identity drift' unless Digest::SHA256.file(p).hexdigest==h }
oracle=JSON.parse(File.read(state+'/oracle.json'));raise 'oracle coverage' unless oracle.size==20
raise 'mode coverage' unless manifest['runs'].map { |r| r['mode'] }==%w[vm interpreter jit]
manifest['runs'].each do |run|
  raise 'state outcome' unless run['exit']==0 && run['signal'].nil?
  actual=File.readlines(state+'/'+run['mode']+'.out').map { |l| JSON.parse(l) }
  raise 'state/count equality' unless actual==oracle.each_with_index.map { |row,i| [i,row,row] }
end
report=JSON.parse(File.read(timing+'/report.json'))
raise 'unfinished timings' unless report['status']=='collected-review-pending'
report.fetch('files').each { |p,h| raise 'timing identity drift' unless Digest::SHA256.file(p).hexdigest==h }
raise 'fixed work' unless report['iterations']==20
expected=(-1...7).flat_map do |round|
  %w[vm interpreter jit].rotate([round,0].max%3).each_with_index.flat_map do |mode,index|
    ((round+index).even? ? %w[string byte] : %w[byte string]).map { |variant| [round,mode,variant] }
  end
end
raise 'sample membership/order' unless report['rows'].map { |r| r.values_at('round','runtime','variant') }==expected
report['rows'].each do |row|
  raise 'sample outcome' unless row['passed']==true && row['exit']==0 && row['signal'].nil? && row['warmup']==(row['round']==-1)
  raise 'elapsed' unless row['elapsed_seconds'].is_a?(Numeric) && row['elapsed_seconds'].finite? && row['elapsed_seconds']>0
  label=row.values_at('round','runtime','variant').join('-')
  raise 'raw output' unless File.read(timing+'/'+label+'.out')==row['stdout'] && File.read(timing+'/'+label+'.err')==row['stderr'] && row['stdout'].strip==report['expected'].to_s
  argv=[report['files'].keys.first,'run','--scheduler-timeout-ms','120000'];argv<<"--#{row['runtime']}" unless row['runtime']=='vm';argv<<timing+'/'+row['variant']+'.kujo'
  raise 'invocation' unless row['argv']==argv
end
comparisons=%w[vm interpreter jit].map do |mode|
  ratios=(0...7).map do |round|
    pair=report['rows'].select { |r| r['round']==round && r['runtime']==mode }.to_h { |r| [r['variant'],r['elapsed_seconds']] }
    pair['string']/pair['byte']
  end
  {'runtime'=>mode,'paired_speedups'=>ratios,'median'=>ratios.sort[3],'range'=>ratios.minmax}
end
raise 'ratios' unless comparisons==report['comparisons']
File.write(timing+'/verified.json',JSON.pretty_generate({status:'verified-diagnostic-not-accepted',full_states:120,samples:48,warmups:6,comparisons:comparisons,scope:report['scope']}))
puts JSON.pretty_generate(comparisons)
