# Explicit pool-regime acceptance; the original exact-equality rejection is retained.
require 'json'
require 'digest'
abort 'Expected ORIGINAL_REPORT EXTENDED_REPORT NEW_OUTPUT' unless ARGV.size == 3
original, extended, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
reports = [original, extended].map { |p| JSON.parse(File.read(p)) }
width_sets = [[0, 128, 2048, 8192], [4096, 8192, 12288, 16384]]
key = ->(r) { r.values_at('bindings', 'operation', 'depth', 'calls', 'mode') }
all_deltas = []
reports.zip(width_sets).each do |r, widths|
  raise 'Paired report incomplete' unless r['status'] == 'compared_without_regressions'
  expected = widths.product(%w[read write], [0, 1, 4], [1, 16, 32], %w[vm jit])
  raise 'Allocation coverage' unless r.fetch('rows').map(&key).sort == expected.sort
  semantic = widths.product(%w[read write], [0, 1, 4], [1, 16, 32], %w[vm interpreter jit])
  raise 'Semantic coverage' unless r.fetch('semantics').map(&key).sort == semantic.sort && r['semantics'].all? { |x| x['exit'] == 0 }
  raise 'Total allocation regression' unless r['rows'].all? { |x| %w[requests bytes].all? { |m| x['candidate'][m] <= x['baseline'][m] } }
  deltas = widths.product(%w[read write], [0, 1, 4], %w[vm jit]).map do |width, operation, depth, mode|
    pair = [16, 32].map { |calls| r['rows'].find { |x| key.call(x) == [width, operation, depth, calls, mode] } }
    row = { 'bindings' => width, 'operation' => operation, 'depth' => depth, 'mode' => mode }
    %w[candidate baseline].each { |label| row[label] = %w[requests bytes].to_h { |m| [m, (pair[1][label][m] - pair[0][label][m]) / 16.0] } }
    row
  end
  raise 'Recorded deltas inconsistent' unless deltas == r.fetch('deltas')
  raise 'Added-call allocation regression' unless deltas.all? { |x| %w[requests bytes].all? { |m| x['candidate'][m] <= x['baseline'][m] } }
  all_deltas.concat(deltas)
end
%w[binary_sha256 probe_sha256 baseline_probe_sha256].each { |k| raise 'Artifact identity mismatch' unless reports.map { |r| r.fetch(k) }.uniq.size == 1 }
groups = all_deltas.group_by { |x| x.values_at('bindings', 'operation', 'depth', 'mode') }
raise 'Repeated-width deltas disagree' unless groups.values.all? { |g| g.uniq.size == 1 }
deltas = groups.values.map(&:first)
widths = width_sets.flatten.uniq.sort
raise 'Combined coverage' unless deltas.size == widths.size * 2 * 3 * 2
overheads = widths.product(%w[read write], [1, 4], %w[vm jit]).map do |width, operation, depth, mode|
  pair = [0, depth].map { |level| deltas.find { |x| x.values_at('bindings', 'operation', 'depth', 'mode') == [width, operation, level, mode] } }
  row = { 'bindings' => width, 'operation' => operation, 'depth' => depth, 'mode' => mode }
  %w[candidate baseline].each { |label| row[label] = %w[requests bytes].to_h { |m| [m, pair[1][label][m] - pair[0][label][m]] } }
  row
end
regimes = { 'fits_existing_pool' => [0, 128, 2048, 4096], 'exceeds_existing_pool' => [8192, 12288, 16384] }
regimes.each do |name, members|
  groups = overheads.select { |r| members.include?(r['bindings']) }.group_by { |r| r.values_at('operation', 'depth', 'mode') }
  raise 'Recursive cost grows within pool regime ' + name unless groups.values.all? { |g| g.map { |r| r['candidate'] }.uniq.size == 1 }
end
raise 'Recursive overhead regression' unless overheads.all? { |r| %w[requests bytes].all? { |m| r['candidate'][m] <= r['baseline'][m] } }
targets = deltas.select { |r| r['depth'] == 0 && r['bindings'] >= 8192 }
raise 'Outer activation reduction missing' unless targets.size == 12 && targets.all? { |r| r['candidate']['bytes'] < r['baseline']['bytes'] && r['candidate']['requests'] <= r['baseline']['requests'] }
result = { status: 'passed', scope: 'Reduce oversized outer-activation bytes without increasing requests, total allocations, added-call allocations or recursive overhead',
  acceptance: 'Predeclared outer-activation target; accepted recursion scaling preserved',
  runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
  inputs: [original, extended].map { |p| { path: p, sha256: Digest::SHA256.file(p).hexdigest } },
  binary_sha256: reports[0]['binary_sha256'], probe_sha256: reports[0]['probe_sha256'], baseline_probe_sha256: reports[0]['baseline_probe_sha256'],
  allocation_observations: 288, semantic_runs: 432, unique_allocation_cases: 252, unique_semantic_cases: 378,
  repeated_width: 8192, strict_reduction_groups: 12, regimes: regimes, deltas: deltas, overheads: overheads }
File.write(output, JSON.pretty_generate(result) + "\n")
puts 'Verified 12 oversized outer-activation byte reductions and preserved total/added-call/recursion bounds'
