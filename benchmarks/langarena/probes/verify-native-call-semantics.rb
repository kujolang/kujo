# Verify a completed candidate receipt. This does not run or accept a speed test.
require 'json'
require 'digest'

root = File.expand_path(ARGV.fetch(0))
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
manifest = JSON.parse(File.read(root + '/manifest.json'))
manifest.fetch('source_hashes').each do |path, hash|
  raise "source drift: #{path}" unless sha.call(path) == hash
end
inputs = JSON.parse(File.read(root + '/input-identities.json'))
inputs.fetch('expected_sha256').each do |path, hash|
  raise "input drift: #{path}" unless sha.call(path) == hash
end
raise 'candidate binary drift' unless sha.call(root + '/kujo') == manifest.fetch('binary_sha256')
raise 'Cargo CLI differs from frozen candidate' unless sha.call('target/release/kujo') == manifest.fetch('binary_sha256')

state = JSON.parse(File.read(root + '/all50-state.json'))
raise 'incomplete checksum run' unless state['status'] == 'passed' && state['verified_checksums'] == 150
cases = JSON.parse(File.read('benchmarks/langarena/test.json'))
raise 'registry size' unless cases.size == 50 && cases.map { |c| c['name'] }.uniq.size == 50
raise 'runtime command coverage' unless state.fetch('commands').map { |c| c['mode'] } == %w[vm jit interpreter]
state.fetch('commands').each do |command|
  raise 'driver failed' unless command['exit'] == 0 && command['signal'].nil?
end
%w[vm jit interpreter].each do |mode|
  report = JSON.parse(File.read(root + "/all50-#{mode}.json"))
  rows = report.fetch('results')
  raise 'report identity' unless report['runtime'] == mode && File.realpath(report['binary']) == root + '/kujo'
  raise 'case coverage/order' unless rows.map { |r| r['name'] } == cases.map { |c| c['name'] }
  rows.zip(cases).each do |row, config|
    raise 'case/config failure' unless row['config'] == config && row['runtime'] == mode && row['status'] == 'passed' && row['exitcode'] == 0
    raise 'truncated output' if row['stdout_truncated'] || row['stderr_truncated']
    lines = row['stdout'].lines.map(&:strip)
    raise 'checksum' unless lines.count("#{config['name']}: OK #{config['checksum']}") == 1 && lines.count('Summary: 1, 1, 0') == 1
  end
end

results = {}
{'contracts' => 3, 'owned-tests' => 2, 'environment-tests' => 3, 'task-tests' => 3}.each do |name, count|
  log = File.read(root + "/#{name}.log")
  expected = "test result: ok. #{count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
  raise "targeted suite incomplete: #{name}" unless log.include?(expected)
  results[name] = {passed: count, binary_sha256: sha.call(root + '/' + name), log_sha256: Digest::SHA256.hexdigest(log)}
end

# These fixed counts match the current source with runtime-jit enabled. An
# added/removed/ignored test requires review, not automatic receipt adjustment.
expected = {'jit_immediate_arguments' => 8, 'jit_loop_condition_calls' => 13, 'vm_interpreter_parity_surfaces' => 123}
log = File.read(root + '/remaining-regressions.log')
sections = log.split(/\s+Running tests\//).drop(1)
raise 'missing/extra Cargo suites' unless sections.size == expected.size
sections.each do |section|
  match = section.match(/\A([^\s]+)\.rs \(([^)]+)\)/) or raise 'suite identity'
  name, binary = match.captures
  count = expected.fetch(name)
  summary = "test result: ok. #{count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
  raise "Cargo suite incomplete: #{name}" unless section.include?(summary)
  raise 'test row count' unless section.scan(/^test .+ \.\.\. ok$/).size == count
  raise 'duplicate suite' if results.key?(name)
  results[name] = {passed: count, binary: binary, binary_sha256: sha.call(binary)}
end
raise 'Cargo suite coverage' unless (results.keys & expected.keys).sort == expected.keys.sort
focused = File.read(File.join(root, '..', 'call-contract-candidate-test.log'))
raise 'original Cargo gate incomplete' unless focused.include?('test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;')

output = {
  status: 'verified_review_pending', scope: 'Call-error semantic prerequisite; no performance or production claim',
  binary_sha256: manifest.fetch('binary_sha256'), checksum_cases: 150,
  unique_tests_passed: results.values.sum { |r| r.fetch(:passed) }, suites: results,
  original_cargo_focused_rerun_passed: 3, rerun_counted_in_unique_tests: false,
  source_identities: manifest.fetch('source_hashes').size,
  input_identity_receipt_phase: inputs.fetch('phase'),
  remaining_log_sha256: sha.call(root + '/remaining-regressions.log'),
  verifier_sha256: sha.call(__FILE__)
}
File.write(root + '/verified.json', JSON.pretty_generate(output) + "\n")
puts JSON.generate(output.reject { |key, _| key == :suites })
