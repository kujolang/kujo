# Independently validate a same-build baseline for row overflow work.
require 'json'
require 'digest'
abort 'Expected MATRIX_REPORT' unless ARGV.size == 1
path = File.expand_path(ARGV[0]); report = JSON.parse(File.read(path))
raise 'Incomplete run' unless report['status'] == 'passed'
runner = File.join(__dir__, 'row-overflow-isolation.rb')
raise 'Runner drift' unless Digest::SHA256.file(runner).hexdigest == report['runner_sha256']
report['artifacts'].each { |p, h| raise 'Artifact drift' unless Digest::SHA256.file(p).hexdigest == h }
keys = %w[shape width writes snapshots style calls mode]
cases = %w[struct dictionary array].product([128, 2048], [4, 5, 16], %w[once fresh], %w[loop explicit], [16, 32])
rows = report.fetch('rows'); semantics = report.fetch('semantics')
raise 'Allocation coverage' unless rows.map { |r| r.values_at(*keys) }.sort == cases.product(%w[vm jit]).map { |c, m| c + [m] }.sort
raise 'Semantic coverage' unless semantics.map { |r| r.values_at(*keys) }.sort == cases.product(%w[vm interpreter jit]).map { |c, m| c + [m] }.sort
report['sources'].each { |h, source| raise 'Source hash' unless Digest::SHA256.hexdigest(source) == h }
(rows + semantics).each do |r|
  dir = File.join(File.dirname(path), r.values_at('shape', 'width', 'writes', 'snapshots', 'style').join('-'))
  {'source_sha256' => "calls-#{r['calls']}.kujo", 'module_sha256' => 'owner.kujo'}.each do |key, file|
    source = File.read(File.join(dir, file))
    raise 'Fixture identity' unless Digest::SHA256.hexdigest(source) == r[key] && report['sources'][r[key]] == source
  end
  raise 'Semantic failure' if r.key?('exit') && r['exit'] != 0
end
raise 'Calibration mismatch' unless rows.all? { |r| r['baseline'] == r['candidate'] }
deltas = rows.group_by { |r| r.values_at(*keys.reject { |k| k == 'calls' }) }.map do |key, group|
  a, b = group.sort_by { |r| r['calls'] }
  r = keys.reject { |k| k == 'calls' }.zip(key).to_h
  %w[baseline candidate].each { |v| r[v] = %w[requests bytes].to_h { |m| [m, (b[v][m] - a[v][m]) / 16.0] } }
  r
end
raise 'Delta mismatch' unless deltas == report['deltas']
deltas.group_by { |r| r.values_at('shape', 'writes', 'snapshots', 'style', 'mode') }.each do |key, group|
  a, b = group.sort_by { |r| r['width'] }
  if key[1] == 4 && key[2] == 'once'
    raise 'Inline width dependence' unless a['candidate'] == b['candidate']
  elsif key[1] > 4 && key[2] == 'once'
    raise 'Overflow cost not reproduced' unless b['candidate']['bytes'] > a['candidate']['bytes']
  end
end
deltas.group_by { |r| r.values_at('shape', 'width', 'writes', 'snapshots', 'style') }.each do |key, group|
  raise 'VM/JIT disagreement' unless group.size == 2 && group.map { |r| r['candidate'] }.uniq.size == 1
end
puts "Verified #{rows.size} calibrated allocation pairs and #{semantics.size} semantic cases; row overflow reproduced in both source forms"
