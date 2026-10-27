# Acceptance for the predeclared foreign-local entry-copying optimization.
require 'json'
require 'digest'
abort 'Expected PAIRED_REPORT NEW_ACCEPTANCE_JSON' unless ARGV.size == 2
input, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
r = JSON.parse(File.read(input))
raise 'Paired measurement incomplete' unless r['status'] == 'compared_without_regressions'
key = ->(row) { row.values_at('bindings', 'operation', 'depth', 'calls', 'mode') }
expected = [0, 128, 2048, 8192].product(%w[read write], [0, 1, 4], [1, 16, 32], %w[vm jit])
raise 'Allocation coverage' unless r.fetch('rows').map(&key).sort == expected.sort
semantics = [0, 128, 2048, 8192].product(%w[read write], [0, 1, 4], [1, 16, 32], %w[vm interpreter jit])
raise 'Semantic coverage' unless r.fetch('semantics').map(&key).sort == semantics.sort && r['semantics'].all? { |row| row['exit'] == 0 }
raise 'Total allocation regression' unless r['rows'].all? { |row| %w[requests bytes].all? { |metric| row['candidate'].fetch(metric) <= row['baseline'].fetch(metric) } }
deltas = [0, 128, 2048, 8192].product(%w[read write], [0, 1, 4], %w[vm jit]).map do |bindings, operation, depth, mode|
  pair = [16, 32].map { |calls| r['rows'].find { |row| key.call(row) == [bindings, operation, depth, calls, mode] } }
  row = { 'bindings' => bindings, 'operation' => operation, 'depth' => depth, 'mode' => mode }
  %w[candidate baseline].each { |label| row[label] = %w[requests bytes].to_h { |metric| [metric, (pair[1][label][metric] - pair[0][label][metric]) / 16.0] } }
  row
end
raise 'Inconsistent measured deltas' unless deltas == r.fetch('deltas')
raise 'Per-call regression' unless deltas.all? { |row| %w[requests bytes].all? { |metric| row['candidate'][metric] <= row['baseline'][metric] } }
overheads = [0, 128, 2048, 8192].product(%w[read write], [1, 4], %w[vm jit]).map do |bindings, operation, depth, mode|
  pair = [0, depth].map { |level| deltas.find { |row| row.values_at('bindings', 'operation', 'depth', 'mode') == [bindings, operation, level, mode] } }
  row = { 'bindings' => bindings, 'operation' => operation, 'depth' => depth, 'mode' => mode }
  %w[candidate baseline].each { |label| row[label] = %w[requests bytes].to_h { |metric| [metric, pair[1][label][metric] - pair[0][label][metric]] } }
  row
end
raise 'Foreign recursive overhead still scales' unless overheads.group_by { |row| row.values_at('operation', 'depth', 'mode') }.values.all? { |group| group.map { |row| row['candidate'] }.uniq.size == 1 }
targets = overheads.select { |row| row['depth'] == 4 && row['bindings'] > 0 }
raise 'Foreign entry costs not reduced' unless targets.size == 12 && targets.all? { |row| %w[requests bytes].all? { |metric| row['candidate'][metric] < row['baseline'][metric] } }
result = { status: 'passed', scope: 'Foreign local capture reentry; outer activation and split-frame fallback remain separate',
  input_sha256: Digest::SHA256.file(input).hexdigest, runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
  binary_sha256: r['binary_sha256'], probe_sha256: r['probe_sha256'], baseline_probe_sha256: r['baseline_probe_sha256'],
  allocation_pairs: 144, semantic_runs: 216, strict_reduction_groups: 12, overheads: overheads }
File.write(output, JSON.pretty_generate(result) + "\n")
puts 'Foreign prefix gate passed: constant overhead at depths one/four and twelve strict reductions'
