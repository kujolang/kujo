require 'json'
require 'digest'
require 'open3'
root = File.expand_path(ARGV.fetch(0))
read = ->(name) { JSON.parse(File.read(File.join(root, name))) }
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
r = read.call('report.json')
raise 'unfinished census' unless r['status'] == 'measured'
manifest = read.call('source-manifest.json')
manifest.each { |path, hash| raise 'instrumented source drift' unless sha.call(path) == hash }
overlay = read.call('instrumentation.json')
isolated = File.dirname(manifest.keys.find { |p| p.end_with?('/Cargo.toml') })
raise 'build completion missing' unless File.read(root+'/build-final.log').match?(/Finished `release` profile/)
overlay['edits'].group_by { |edit| edit['path'] }.each do |path, edits|
  original, status = Open3.capture2('git', 'show', "#{overlay['baseline']}:#{path}")
  raise 'overlay baseline' unless status.success? && Digest::SHA256.hexdigest(original) == edits.first['before_sha256']
  edits.each_cons(2) { |a, b| raise 'overlay chain' unless a['after_sha256'] == b['before_sha256'] }
  raise 'overlay result' unless manifest[isolated+'/'+path] == edits.last['after_sha256']
end
r['sources'].each { |path, hash| raise 'source drift' unless sha.call(path) == hash }
r['artifacts'].each { |path, hash| raise 'artifact drift' unless sha.call(File.join(root, path)) == hash }
config = r['sources'].keys.find { |p| p.end_with?('/benchmarks/langarena/test.json') }
cases = JSON.parse(File.read(config))
expected = cases.flat_map { |c| %w[vm interpreter jit].map { |mode| [c['name'], mode] } }
raise 'coverage' unless r['rows'].map { |x| x.values_at('case', 'mode') } == expected && expected.size == 150
raise 'labels' unless r['labels'] == read.call('instrumentation.json')['labels']
r['rows'].each do |row|
  c = cases.find { |x| x['name'] == row['case'] }
  raise 'argv' unless row['argv'] == [root+'/capture-census', row['mode'], row['case']]
  lines = row['stdout'].lines.map(&:strip)
  raise 'checksum' unless row['exit'] == 0 && lines.count("#{c['name']}: OK #{c['checksum']}") == 1 && lines.count('Summary: 1, 1, 0') == 1
  counters = row['stderr'].scan(/^CAPTURE_CENSUS (\[.*\])$/)
  raise 'counter receipt' unless counters.size == 1 && JSON.parse(counters[0][0]) == row['counts']
  raise 'counter shape' unless row['counts'].size == r['labels'].size && row['counts'].all? { |v| v.is_a?(Integer) && v >= 0 }
end
controls = read.call('positive-controls.json')
raise 'control identity' unless controls['status'] == 'passed' && controls['binary_sha256'] == sha.call(root+'/capture-census')
controls['sources'].each { |path, hash| raise 'control source drift' unless sha.call(path) == hash }
raise 'control coverage' unless controls['rows'].map{|x|x.values_at('calls','mode')} == %w[read-1 read-32].flat_map{|c| %w[vm interpreter jit].map{|m|[c,m]}}
controls['rows'].each do |row|
  counts=row['stderr'].scan(/^CAPTURE_CENSUS (\[.*\])$/)
  raise 'positive control' unless row['exit']==0 && row['stdout'].strip=='LOCAL CAPTURE PASS' && counts.size==1 && JSON.parse(counts[0][0])==row['counts'] && row['counts'][0]>0 && row['counts'][3]>0 && row['counts'][4]>=8192
end
bridge = read.call('bridge-controls.json')
raise 'bridge identity' unless bridge['status'] == 'measured' && bridge['binary_sha256'] == sha.call(root+'/capture-census')
bridge['sources'].each { |path, hash| raise 'bridge source drift' unless sha.call(path) == hash }
raise 'bridge coverage' unless bridge['rows'].map{|x|x.values_at('kind','mode')} == %w[write error read].flat_map{|c|%w[vm interpreter jit].map{|m|[c,m]}}
bridge['rows'].each do |row|
  counts = row['stderr'].scan(/^CAPTURE_CENSUS (\[.*\])$/)
  raise 'bridge semantics' unless row['exit']==0 && row['stdout'].strip=='BRIDGE PASS' && counts.size==1 && JSON.parse(counts[0][0])==row['counts']
  raise 'prefix control' unless row['counts'][5]>0 && row['counts'][6]>0
  route = row['mode']=='interpreter' ? 9 : 8
  raise 'fallback route control' unless row['counts'][route]>0
end
summary = r['labels'].each_with_index.to_h do |label, i|
  [label, r['rows'].select { |row| row['counts'][i] > 0 }.map { |row| {case: row['case'], mode: row['mode'], count: row['counts'][i]} }]
end
File.write(root+'/verified.json', JSON.pretty_generate({status:'passed', checksum_runs:150, positive_controls:15, report_sha256:sha.call(root+'/report.json'), controls_sha256:sha.call(root+'/positive-controls.json'), bridge_controls_sha256:sha.call(root+'/bridge-controls.json'), reached:summary, limits:'Test-size reachability only; not production timing, allocation attribution or proof of unreachable behavior at every input size. Pending-publication clone counter has no positive control in this corpus.'})+"\n")
puts '150 checksum runs and fifteen positive controls verified'
