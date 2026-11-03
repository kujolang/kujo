require 'json'
require 'digest'
raise 'usage: TIMING_DIRECTORY' unless ARGV.size==1
out=File.expand_path(ARGV[0]);report=JSON.parse(File.read(out+'/report.json'))
raise 'unfinished' unless report['status']=='collected-review-pending'
report.fetch('files').each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
raise 'work counts' unless report.values_at('warmup_calls','measured_calls')==[120,20]
%w[string byte].each do |variant|
  source=File.read(out+'/'+variant+'.kujo')
  raise 'phase boundaries' unless source.scan('while iteration < 120').size==1 && source.scan('while iteration < 20').size==1 && source.index('while iteration < 120')<source.index('let started := time_ns()') && source.index('let started := time_ns()')<source.index('while iteration < 20')
end
expected=(0...7).flat_map { |round| (round.even? ? %w[string byte] : %w[byte string]).map { |variant| [round,variant] } }
raise 'membership/order' unless report['rows'].map { |r| r.values_at('round','variant') }==expected
report['rows'].each do |row|
  raise 'outcome' unless row['passed']==true && row['exit']==0 && row['signal'].nil?
  raw=File.read(out+"/#{row['round']}-#{row['variant']}.out");err=File.read(out+"/#{row['round']}-#{row['variant']}.err");data=JSON.parse(raw)
  raise 'raw output' unless raw==row['stdout'] && err==row['stderr'] && data['result']==report['expected'] && data['elapsed_ns']/1e9==row['kernel_seconds']
  raise 'timing' unless row['kernel_seconds']>0 && row['process_seconds']>=row['kernel_seconds']
  argv=[report['files'].keys.first,'run','--jit','--scheduler-timeout-ms','120000',out+'/'+row['variant']+'.kujo']
  raise 'invocation' unless row['argv']==argv
end
comparisons=%w[kernel_seconds process_seconds].map do |field|
  ratios=7.times.map do |round|
    pair=report['rows'].select { |r| r['round']==round }.to_h { |r| [r['variant'],r[field]] }
    pair['string']/pair['byte']
  end
  {'metric'=>field,'paired_speedups'=>ratios,'median'=>ratios.sort[3],'range'=>ratios.minmax}
end
raise 'ratios' unless comparisons==report['comparisons']
File.write(out+'/verified.json',JSON.pretty_generate({status:'verified-diagnostic-not-accepted',samples:14,comparisons:comparisons,scope:report['scope']}))
puts JSON.pretty_generate(comparisons)
