require 'json'
require 'digest'
require 'open3'
raise 'usage: ARTIFACT_DIRECTORY CANDIDATE_SHA [HARNESS_SHA [BASELINE_SHA]]' unless (2..4).cover?(ARGV.size)
root, candidate, harness, baseline = ARGV
harness ||= candidate
raise 'harness identity' unless harness.match?(/\A[0-9a-f]{40}\z/)
raise 'candidate identity' unless candidate.match?(/\A[0-9a-f]{40}\z/) && File.read(root + '/candidate-source.txt').strip == candidate
baseline ||= 'cd5f2b207a8f907d6c263f5a2e405412965344ca'
raise 'expected baseline identity' unless baseline.match?(/\A[0-9a-f]{40}\z/)
raise 'baseline identity' unless File.read(root + '/baseline-source.txt').strip == baseline
report = JSON.parse(File.read(root + '/timings/report.json'))
raise 'incomplete collection' unless report['status'] == 'collected-review-pending'
read_source = ->(path) do
  value, error, status = Open3.capture3('git', 'show', "#{harness}:#{path}")
  raise error unless status.success?
  value
end
config = JSON.parse(read_source.call('benchmarks/langarena/test.json')).to_h { |r| [r.fetch('name'), r] }
parts = read_source.call('benchmarks/langarena/build.kujo')[/let parts := (\[[^\n]+\])/, 1]
raise 'assembly list' unless parts
source = JSON.parse(parts).map { |part| read_source.call("benchmarks/langarena/src/#{part}.kujo") + "\n" }.join
first = '            while i < cfg["iterations"] {'
last = '            let actual := u32(checksum(state))'
raise 'anchors' unless source.scan(first).size == 1 && source.scan(last).size == 1
source = source.sub(first, "            let comparison_started := time_ns()\n" + first).sub(last, "            print(\"@comparison_ns \" + str(time_ns() - comparison_started))\n" + last)
raise 'changed timed work' unless File.read(root + '/timings/timed-port.kujo') == source
raise 'unexpected inputs' unless report.fetch('files').size == 5
config_path = report['files'].keys.find { |p| p.end_with?('/benchmarks/langarena/test.json') }
port_path = report['files'].keys.find { |p| p.end_with?('/timed-port.kujo') }
collector_path = report['files'].keys.find { |p| p.end_with?('/probes/compare-local-store-targets.rb') }
raise 'configuration identity' unless report['files'][config_path] == Digest::SHA256.hexdigest(read_source.call('benchmarks/langarena/test.json'))
raise 'timed port identity' unless report['files'][port_path] == Digest::SHA256.hexdigest(source)
raise 'collector identity' unless report['files'][collector_path] == Digest::SHA256.hexdigest(read_source.call('benchmarks/langarena/probes/compare-local-store-targets.rb'))
names = %w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::TextRaytracer Matmul::T4 Matmul::T8 Matmul::T16 Base64::Encode Base64::Decode Etc::CacheSimulation Maze::Generator Maze::BFS Maze::AStar Compress::BWTEncode Compress::HuffEncode Compress::HuffDecode]
cells = names.product(%w[vm jit]); raise 'cells' unless report['cells'] == cells
expected = (-1...7).flat_map do |round|
  cells.rotate([round, 0].max).flat_map do |cell|
    ((round + cells.index(cell)).even? ? %w[baseline candidate] : %w[candidate baseline]).map { |variant| [round, *cell, variant] }
  end
end
rows = report.fetch('rows')
raise 'sample coverage/order' unless rows.map { |r| r.values_at('round', 'name', 'runtime', 'variant') } == expected
%w[baseline candidate].each do |variant|
  raise 'binary hash' unless Digest::SHA256.file(root + '/' + variant + '-kujo').hexdigest == report.fetch('files').fetch(report.fetch('binaries').fetch(variant))
  build = File.readlines(root + '/' + variant + '-build.jsonl').map { |line| JSON.parse(line) }
  raise 'build outcome' unless build.select { |r| r['reason'] == 'build-finished' }.map { |r| r['success'] } == [true]
  target = build.select { |r| r['reason'] == 'compiler-artifact' && r.dig('target', 'name') == 'kujo' && r.dig('target', 'kind') == ['bin'] }
  raise 'release build' unless target.size == 1 && target[0].dig('profile', 'opt_level') == '3' && target[0].dig('profile', 'test') == false
end
rows.each do |row|
  argv = [report['binaries'].fetch(row['variant']), 'run', '--scheduler-no-timeout']
  argv << '--jit' if row['runtime'] == 'jit'
  argv += [port_path, config_path, row['name']]
  raise 'invocation' unless row['argv'] == argv
  label = [row['round'], cells.index(row.values_at('name', 'runtime')), row['variant']].join('-')
  stdout = File.read(root + '/timings/' + label + '.out'); stderr = File.read(root + '/timings/' + label + '.err')
  raise 'raw mismatch' unless row['stdout'] == stdout && row['stderr'] == stderr && row['exit'] == 0
  timer = stdout.lines.grep(/^@comparison_ns /)
  raise 'checksum/timer' unless timer.size == 1 && stdout.lines.map(&:strip) == ["@comparison_ns #{row['kernel_ns']}", "#{row['name']}: OK #{config.fetch(row['name']).fetch('checksum')}", 'Summary: 1, 1, 0']
  raise 'stderr' unless stderr.empty? || (row['runtime'] == 'jit' && stderr.match?(/\AJIT opt-in requested; unsupported bytecode regions will use the VM \([^\n]+\)\.\n\z/))
  raise 'positive timings' unless row['kernel_ns'].is_a?(Integer) && row['kernel_ns'] > 0 && row['wall_seconds'].is_a?(Numeric) && row['wall_seconds'].finite? && row['wall_seconds'] > 0 && row['wall_seconds'] < 301
end
summary = cells.map do |name, runtime|
  group = rows.select { |r| r['round'] >= 0 && r['name'] == name && r['runtime'] == runtime }
  metrics = %w[kernel_ns wall_seconds].to_h do |metric|
    pairs = (0...7).map do |round|
      cell = group.select { |r| r['round'] == round }.to_h { |r| [r['variant'], r.fetch(metric)] }
      cell.fetch('baseline').to_f / cell.fetch('candidate')
    end
    raw = %w[baseline candidate].to_h { |v| [v, group.select { |r| r['variant'] == v }.map { |r| r.fetch(metric) }] }
    [metric, {raw: raw, paired_speedups: pairs, median_speedup: pairs.sort[3], range: pairs.minmax, candidate_faster_pairs: pairs.count { |x| x > 1 }}]
  end
  {name: name, runtime: runtime, metrics: metrics}
end
result = {status: 'timings-verified-review-pending', candidate: candidate, harness: harness, samples: rows.size, warmups: rows.count { |r| r['round'] == -1 }, comparisons: summary,
          scope: 'Actual source/timing boundaries, binary bytes, build outcomes, alternating order, raw outputs and recomputed ratios. Correctness logs, resource review, independent-session agreement and acceptance remain separate.'}
File.write(root + '/timing-verification.json', JSON.pretty_generate(result))
puts JSON.pretty_generate(result)
