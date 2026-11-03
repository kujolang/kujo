# Validate every raw result before summarizing paired call diagnostics.
require 'json'
require 'digest'
abort 'usage: ruby verify-native-calls.rb OUTPUT' unless ARGV.length == 1
directory = File.expand_path(ARGV[0])
state = JSON.parse(File.read(File.join(directory, 'state.json')))
raise 'incomplete comparison' unless state.fetch('status') == 'measured-review-pending'
rounds = state.fetch('rounds')
raise 'invalid rounds/calls' unless rounds.is_a?(Integer) && rounds > 0 && state.fetch('calls') == 32768
state.fetch('files').each do |path, hash|
  raise "identity drift: #{path}" unless Digest::SHA256.file(path).hexdigest == hash
end
cells = (0..4).map { |a| ['scalar', a, 8] } + (1..4).to_a.product([8, 256, 8192]).map { |a, w| ['array', a, w] }
rows = state.fetch('rows')
raise 'coverage count' unless rows.length == cells.length * rounds * 4
index = {}
admission_notice = "JIT opt-in requested; unsupported bytecode regions will use the VM (unsupported opcode in chunk '<script>' at instruction 0: MakeClosure(0)).\n"
rows.each do |row|
  cell = row.values_at('kind', 'arity', 'width')
  revision, mode, round = row.values_at('revision', 'runtime', 'round')
  raise 'invalid cell' unless cells.include?(cell) && %w[baseline candidate].include?(revision) && %w[vm jit].include?(mode) && (1..rounds).cover?(round)
  key = cell + [revision, mode, round]
  raise 'duplicate sample' if index.key?(key)
  raise 'failed sample' unless row['passed'] == true && row['exit'] == 0 && row['signal'].nil? && row['timed_out'] == false
  %w[stdout stderr].each do |stream|
    path = row.fetch(stream)
    raise 'output outside evidence directory' unless File.dirname(File.expand_path(path)) == directory
    raise "#{stream} hash mismatch" unless Digest::SHA256.file(path).hexdigest == row.fetch(stream + '_sha256')
  end
  output = File.read(row.fetch('stdout'))
  match = /\A@call_ns ([0-9]+)\r?\nnative-call-scaling: OK\r?\n\z/.match(output)
  diagnostics = File.read(row.fetch('stderr'))
  diagnostics_valid = diagnostics.empty? || (mode == 'jit' && diagnostics == admission_notice)
  raise 'output mismatch' unless match && diagnostics_valid
  ns = Integer(match[1])
  raise 'timing mismatch' unless ns > 0 && ns == row.fetch('kernel_ns')
  argv = row.fetch('argv')
  raise 'unknown executable' unless state.fetch('files').key?(argv[0])
  expected = [argv[0], 'run'] + (mode == 'jit' ? ['--jit'] : []) + ['benchmarks/langarena/probes/native-call-scaling.kujo', cell[0], cell[1].to_s, cell[2].to_s, '32768', 'measure']
  raise 'command mismatch' unless argv == expected
  index[key] = ns
end
# Each label must refer to exactly one executable throughout the run.
executables = rows.group_by { |r| r.fetch('revision') }.transform_values { |rs| rs.map { |r| r.fetch('argv')[0] }.uniq }
raise 'executable label drift' unless executables.values.all? { |paths| paths.length == 1 }
median = lambda do |values|
  sorted = values.sort
  middle = sorted.length / 2
  sorted.length.odd? ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2.0
end
summary = cells.flat_map do |cell|
  %w[vm jit].map do |mode|
    baseline = (1..rounds).map { |r| index.fetch(cell + ['baseline', mode, r]) }
    candidate = (1..rounds).map { |r| index.fetch(cell + ['candidate', mode, r]) }
    ratios = baseline.zip(candidate).map { |before, after| before.to_f / after }
    {kind: cell[0], arity: cell[1], width: cell[2], runtime: mode,
     baseline_median_ns: median.call(baseline), candidate_median_ns: median.call(candidate),
     paired_speedup_median: median.call(ratios), paired_speedup_min: ratios.min,
     paired_speedup_max: ratios.max, candidate_faster_pairs: ratios.count { |r| r > 1 }, pairs: rounds}
  end
end
puts JSON.pretty_generate({status: 'verified-diagnostic-only', samples: rows.length,
                           scope: 'Local call controls; not whole-suite speed or clean-host attribution',
                           executables: executables, summary: summary})
