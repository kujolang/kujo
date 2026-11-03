# Verify an extracted elapsed artifact against pinned source and uploaded bytes.
require 'json'
require 'digest'
require 'open3'
raise 'usage: ARTIFACT_DIRECTORY CANDIDATE_SHA' unless ARGV.size == 2
directory, candidate = ARGV
baseline = 'cd5f2b207a8f907d6c263f5a2e405412965344ca'
raise 'candidate SHA' unless candidate.match?(/\A[0-9a-f]{40}\z/)
report = JSON.parse(File.read(directory + '/samples/report.json'))
raise 'collection incomplete' unless report['status'] == 'collected-review-pending'
%w[baseline candidate].zip([baseline, candidate]).each do |variant, revision|
  raise 'source revision' unless File.read(directory + '/' + variant + '-source.txt').strip == revision
  binary = directory + '/' + variant + '-kujo'
  recorded_path = report.fetch('binaries').fetch(variant)
  raise 'binary hash' unless Digest::SHA256.file(binary).hexdigest == report.fetch('files').fetch(recorded_path)
  build = File.readlines(directory + '/' + variant + '-build.jsonl').map { |line| JSON.parse(line) }
  raise 'build outcome' unless build.select { |row| row['reason'] == 'build-finished' }.map { |row| row['success'] } == [true]
  targets = build.select { |row| row['reason'] == 'compiler-artifact' && row.dig('target', 'name') == 'kujo' && row.dig('target', 'kind') == ['bin'] }
  raise 'optimized binary build' unless targets.size == 1 && targets[0].dig('profile', 'opt_level') == '3' && targets[0].dig('profile', 'test') == false && targets[0]['executable']
end
%w[benchmarks/langarena/probes/jit-async-worker-economics.kujo benchmarks/langarena/probes/compare-owner-elapsed.rb].each do |relative|
  source, error, status = Open3.capture3('git', 'show', "#{candidate}:#{relative}")
  raise error unless status.success?
  paths = report.fetch('files').keys.select { |path| path.end_with?('/' + relative) }
  raise 'source hash' unless paths.size == 1 && Digest::SHA256.hexdigest(source) == report['files'][paths.first]
end
raise 'unexpected inputs' unless report['files'].size == 4
cells = %w[vm interpreter jit].flat_map { |runtime| [['direct',1],['parallel',4],['parallel',8],['parallel',16]].map { |mode,workers| [runtime,mode,workers] } }
expected = (-1...7).flat_map do |round|
  cells.rotate([round,0].max % cells.size).flat_map do |cell|
    ((round+cells.index(cell)).even? ? %w[baseline candidate] : %w[candidate baseline]).map { |variant| [round,*cell,variant] }
  end
end
rows = report.fetch('rows')
raise 'sample membership/order' unless rows.map { |r| r.values_at('round','runtime','mode','workers','variant') } == expected
rows.each do |row|
  raise 'sample outcome' unless row['passed'] == true && row['exit'] == 0 && row['signal'].nil? && row['timed_out'] == false && row['warmup'] == (row['round'] == -1)
  elapsed = row['process_wall_seconds']
  raise 'elapsed value' unless elapsed.is_a?(Numeric) && elapsed.finite? && elapsed > 0 && elapsed < 121
  label = row.values_at('round','runtime','mode','workers','variant').join('-')
  out = File.read(directory + '/samples/' + label + '.out')
  err = File.read(directory + '/samples/' + label + '.err')
  normal_jit_notice = row['runtime'] == 'jit' && err.match?(/\AJIT opt-in requested; unsupported bytecode regions will use the VM \([^\n]+\)\.\n\z/)
  raise 'raw result' unless out == row['stdout'] && err == row['stderr'] && out.lines.map(&:strip) == ['async-worker-economics: OK'] && (err.empty? || normal_jit_notice)
  argv = [report['binaries'][row['variant']], 'run', '--scheduler-timeout-ms', '60000']
  argv << '--interpreter' if row['runtime'] == 'interpreter'
  argv << '--jit' if row['runtime'] == 'jit'
  source = report['files'].keys.find { |p| p.end_with?('/jit-async-worker-economics.kujo') }
  argv += [source, row['mode'], row['workers'].to_s, '4']
  raise 'invocation' unless row['argv'] == argv
end
comparisons = cells.map do |runtime,mode,workers|
  ratios = (0...7).map do |round|
    pair = rows.select { |r| r.values_at('round','runtime','mode','workers') == [round,runtime,mode,workers] }.to_h { |r| [r['variant'],r['process_wall_seconds']] }
    pair.fetch('baseline') / pair.fetch('candidate')
  end
  {'runtime'=>runtime,'mode'=>mode,'workers'=>workers,'paired_speedups'=>ratios,'median_speedup'=>ratios.sort[3],'range'=>ratios.minmax}
end
raise 'reported ratios' unless comparisons == report['comparisons'] && comparisons == JSON.parse(File.read(directory + '/comparisons.json'))
raise 'binary recheck' unless File.read(directory + '/binary-recheck.txt').lines.map(&:strip).sort == ['elapsed-evidence/baseline-kujo: OK','elapsed-evidence/candidate-kujo: OK']
result = {status:'verified-observations-review-pending',baseline:baseline,candidate:candidate,samples:192,warmups:24,comparisons:comparisons,
          scope:'Pinned source, uploaded executable hashes, build records, exact samples and recomputed ratios; workflow provenance and performance acceptance require separate review.'}
File.write(directory + '/verification.json',JSON.pretty_generate(result))
puts JSON.pretty_generate(result)
