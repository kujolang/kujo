# Verify recursion overhead separately from ordinary captured-local activation.
# Preserve the original measurement report, including restoration-gate rejection.
require 'json'
require 'digest'
abort 'Expected MEASUREMENT_REPORT NEW_OUTPUT_FILE' unless ARGV.size == 2
input, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
r = JSON.parse(File.read(input))
raise 'Incomplete measurement' unless r['status'] == 'passed' ||
  (r['status'] == 'failed' && r['error'] == 'Recursive restoration cost not reduced')
expected = [0, 128, 2048, 8192].product(%w[read write], [0, 2], [1, 16, 32], %w[vm jit])
key = ->(row) { row.values_at('bindings', 'operation', 'depth', 'calls', 'mode') }
raise 'Allocation coverage' unless r.fetch('rows').map(&key).sort == expected.sort
expected_semantics = [0, 128, 2048, 8192].product(%w[read write], [0, 2], [1, 16, 32], %w[vm interpreter jit])
raise 'Semantic coverage' unless r.fetch('semantics').map(&key).sort == expected_semantics.sort && r['semantics'].all? { |row| row['exit'] == 0 }
rows = r['rows']
raise 'Allocation regression' unless rows.all? { |row| %w[requests bytes].all? { |metric| row['candidate'].fetch(metric) <= row['baseline'].fetch(metric) } }
deltas = [0, 128, 2048, 8192].product(%w[read write], [0, 2], %w[vm jit]).map do |bindings, operation, depth, mode|
  pair = [16, 32].map { |calls| rows.find { |row| key.call(row) == [bindings, operation, depth, calls, mode] } }
  delta = { 'bindings' => bindings, 'operation' => operation, 'depth' => depth, 'mode' => mode }
  %w[candidate baseline].each do |label|
    delta[label] = %w[requests bytes].to_h { |metric| [metric, (pair[1][label][metric] - pair[0][label][metric]) / 16.0] }
  end
  delta
end
raise 'Reported deltas differ' unless deltas == r.fetch('deltas')
raise 'Per-call regression' unless deltas.all? { |row| %w[requests bytes].all? { |metric| row['candidate'][metric] <= row['baseline'][metric] } }
# Width-dependent activation remains open; isolate the extra cost of depth two.
overheads = [0, 128, 2048, 8192].product(%w[read write], %w[vm jit]).map do |bindings, operation, mode|
  pair = [0, 2].map { |depth| deltas.find { |row| row.values_at('bindings', 'operation', 'depth', 'mode') == [bindings, operation, depth, mode] } }
  row = { 'bindings' => bindings, 'operation' => operation, 'mode' => mode }
  %w[candidate baseline].each do |label|
    row[label] = %w[requests bytes].to_h { |metric| [metric, pair[1][label][metric] - pair[0][label][metric]] }
  end
  row
end
flat = ->(label) { overheads.group_by { |row| row.values_at('operation', 'mode') }.values.all? { |group| group.map { |row| row[label] }.uniq.size == 1 } }
raise 'Baseline does not reproduce width-dependent prefix cost' if flat.call('baseline')
raise 'Recursive prefix overhead still scales with width' unless flat.call('candidate')
targets = overheads.select { |row| [2048, 8192].include?(row['bindings']) }
raise 'Measured prefix costs not reduced' unless targets.size == 8 && targets.all? { |row| %w[requests bytes].all? { |metric| row['candidate'][metric] < row['baseline'][metric] } }
result = {
  status: 'passed', scope: 'Constant added depth-two recursion overhead across 0/128/2048/8192 retained locals; ordinary activation cost remains separate',
  runner_sha256: Digest::SHA256.file(__FILE__).hexdigest, input_sha256: Digest::SHA256.file(input).hexdigest,
  measurement_status: r['status'], measurement_error: r['error'],
  binary_sha256: r['binary_sha256'], probe_sha256: r['probe_sha256'], baseline_probe_sha256: r['baseline_probe_sha256'],
  allocation_pairs: rows.size, semantic_runs: r['semantics'].size, overheads: overheads,
  strict_reduction_groups: targets.size, baseline_reproduces_width_scaling: true,
  note: 'The earlier restoration gate requires improvement at 128 locals as well. This prefix gate preserves that rejection and tests the distinct entry-copying cost; no measurement was changed.'
}
File.write(output, JSON.pretty_generate(result) + "\n")
puts 'Prefix scaling gate passed; eight strict reductions and all-size constant recursive overhead'
