# Independently check complete candidate coverage and retained identities.
require 'json'
require 'digest'
root = File.expand_path(ARGV.fetch(0))
manifest = JSON.parse(File.read(root + '/region-verification-manifest.json'))
hashes = manifest.fetch('files').merge(manifest.fetch('build').fetch('binaries'))
hashes.each { |path, hash| raise "identity drift: #{path}" unless Digest::SHA256.file(path).hexdigest == hash }
cases = JSON.parse(File.read('benchmarks/langarena/test.json'))
raise 'registry coverage' unless cases.size == 50 && cases.map { |c| c['name'] }.uniq.size == 50
reports = %w[vm jit interpreter].map do |mode|
  path = root + "/region-all50-#{mode}.json"
  report = JSON.parse(File.read(path))
  rows = report.fetch('results')
  raise "coverage: #{mode}" unless rows.map { |r| r['name'] } == cases.map { |c| c['name'] }
  raise 'runtime' unless report['runtime'] == mode && rows.all? { |r| r['runtime'] == mode }
  raise 'binary' unless File.realpath(report['binary']) == File.realpath('target/release/kujo')
  rows.zip(cases).each do |row, config|
    raise 'configuration changed' unless row['config'] == config
    raise "failed case: #{mode} #{row['name']}" unless row['status'] == 'passed' && row['exitcode'] == 0
    raise 'truncated evidence' if row['stdout_truncated'] || row['stderr_truncated']
    lines = row['stdout'].lines.map(&:strip)
    raise 'checksum' unless lines.count("#{config['name']}: OK #{config['checksum']}") == 1 && lines.count('Summary: 1, 1, 0') == 1
  end
  {runtime: mode, passed: rows.size, sha256: Digest::SHA256.file(path).hexdigest}
end
controls_path = root + '/region-candidate-controls.json'
controls = JSON.parse(File.read(controls_path))
raise 'control binary' unless controls['binary_sha256'] == manifest['build']['binaries']['target/release/kujo']
expected = %w[contracts boundaries].flat_map do |fixture|
  %w[vm interpreter jit].map do |mode|
    argv = ['target/release/kujo', 'run']
    argv << "--#{mode}" unless mode == 'vm'
    argv << "benchmarks/langarena/probes/native-region-#{fixture}.kujo"
    [argv, "native-region-#{fixture}: OK"]
  end
end
raise 'control coverage' unless controls['rows'].size == expected.size
controls['rows'].zip(expected).each do |row, (argv, output)|
  raise 'control invocation' unless row['argv'] == argv
  raise 'control checksum' unless row['exit'] == 0 && row['stdout'].strip == output
  raise 'control source' unless Digest::SHA256.file(argv.last).hexdigest == row['source_sha256']
end
File.write(root + '/region-checksums-verified.json', JSON.pretty_generate({status: 'passed', checksum_cases: 150, controls: 6, reports: reports, control_report_sha256: Digest::SHA256.file(controls_path).hexdigest, limitations: 'Test-size correctness only; no speed claim. Corrected library unit, remaining integration/resource/production/clean-host gates stay pending.'}) + "\n")
puts 'Verified 150 exact checksum outcomes and six semantic controls with unchanged identities.'
