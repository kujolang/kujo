require 'json'
require 'digest'
require 'open3'
raise 'usage: EVIDENCE_DIRECTORY FULL_COMMIT' unless ARGV.size == 2
out, commit = ARGV
raise 'full commit required' unless commit.match?(/\A[0-9a-f]{40}\z/)
run = JSON.parse(File.read(File.join(out, 'run.json')))
jobs = JSON.parse(File.read(File.join(out, 'jobs.json'))).fetch('jobs')
raise 'run identity/result' unless run['head_sha'] == commit && run['status'] == 'completed' && run['conclusion'] == 'success'
expected_jobs = %w[msrv release-gate-minimal-smoke fmt-check loop-condition-regressions vm-interpreter-parity clippy release-gate]
raise 'job coverage/result' unless jobs.map { |j| j['name'] }.sort == expected_jobs.sort && jobs.all? { |j| j['status'] == 'completed' && j['conclusion'] == 'success' }
source = File.read(File.join(out, 'loop-condition-source.txt'))
raise 'artifact source' unless source.lines.first.strip == commit && source.include?('host: x86_64-unknown-linux-gnu')
source_hashes = source.scan(/^([0-9a-f]{64})  (.+)$/).to_h.invert
raise 'source hash coverage' unless source_hashes.keys.sort == %w[src/jit.rs tests/jit_loop_condition_calls.rs].sort
source_hashes.each do |path, expected|
  bytes, error, status = Open3.capture3('git', 'show', "#{commit}:#{path}")
  raise "source mismatch #{path}: #{error}" unless status.success? && Digest::SHA256.hexdigest(bytes) == expected
end
log = File.read(File.join(out, 'loop-condition-tests.log')).gsub(/\e\[[\d;]*m/, '')
vm_source, error, status = Open3.capture3('git', 'show', "#{commit}:src/vm.rs")
raise "missing VM source: #{error}" unless status.success?
expected_ignored = %w[interpreter::native_functions::http::tests::request_hash_payload_measurement jit::tests::benchmark_cache_lookup_performance jit::tests::benchmark_compilation_overhead jit::tests::benchmark_guard_generation_overhead jit::tests::benchmark_specialization_decision_overhead jit::tests::benchmark_specialized_arithmetic_chain jit::tests::benchmark_specialized_vs_generic_addition jit::tests::benchmark_type_profiling_overhead]
expected_ignored << 'vm::task_owner_resource_tests::task_owner_resource_diagnostic' if vm_source.include?('fn task_owner_resource_diagnostic()')
summaries = log.scan(/^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/).map { |row| row.map(&:to_i) }
raise 'test summaries' unless summaries.size == 9 && summaries.all? { |row| row[1] == 0 && row[3..4] == [0,0] } && summaries.sum { |row| row[2] } == expected_ignored.size
passed = log.scan(/^test (.+) \.\.\. ok$/).flatten
ignored = log.scan(/^test (.+) \.\.\. ignored/).flatten
raise 'test name counts' unless passed.size == summaries.sum { |row| row[0] } && ignored.size == expected_ignored.size

raise 'ignored membership changed' unless ignored.sort == expected_ignored.sort
owner_source = vm_source.split('mod code_owner_lifetime_tests {', 2).fetch(1).split('mod task_owner_resource_tests {', 2).first
owners = owner_source.scan(/#\[test\]\s*fn ([a-zA-Z0-9_]+)\(/).flatten.map { |name| 'vm::code_owner_lifetime_tests::'+name }
raise 'owner source coverage' if owners.empty?
raise 'owner membership' unless owners.all? { |name| passed.count(name) == 1 }
messages = log.lines.select { |line| line.start_with?('{') }.map { |line| JSON.parse(line) }
targets = messages.select { |m| m['reason'] == 'compiler-artifact' && m.dig('profile', 'test') }.map { |m| m.dig('target', 'name') }.sort
expected_targets = %w[kujo jit_loop_condition_calls jit_loop_collection_contracts jit_loop_field_methods jit_loop_owned_rebinding jit_callback_assignment jit_callback_promotion jit_callback_depth jit_error_propagation].sort
raise 'target coverage' unless targets == expected_targets
integration_sources = expected_targets.reject { |target| target == 'kujo' }.to_h do |target|
  path = "tests/#{target}.rs"
  bytes, error, status = Open3.capture3('git', 'show', "#{commit}:#{path}")
  raise "missing integration source: #{error}" unless status.success?
  names = bytes.scan(/^#\[test\]\s*\nfn ([a-zA-Z0-9_]+)\(/).flatten
  raise "unsupported test declarations: #{path}" unless !names.empty? && names.size == bytes.scan(/^#\[test\]/).size
  [path, {sha256:Digest::SHA256.hexdigest(bytes),tests:names}]
end
expected_integration = integration_sources.values.flat_map { |entry| entry[:tests] }.sort
raise 'pinned integration membership' unless passed.reject { |name| name.include?('::') }.sort == expected_integration
raise 'build result' unless messages.select { |m| m['reason'] == 'build-finished' }.map { |m| m['success'] } == [true]
files = %w[run.json jobs.json loop-condition-source.txt loop-condition-tests.log] + jobs.map { |job| "job-#{job['id']}.log" }
hashes = files.to_h { |name| [name, Digest::SHA256.file(File.join(out,name)).hexdigest] }
result = {status:'verified',commit:commit,run:run['id'],jobs:jobs.map { |j| {id:j['id'],name:j['name']} },passed:passed.size,ignored:ignored,owner_tests:owners,targets:targets,source_hashes:source_hashes,integration_sources:integration_sources,files:hashes,scope:'Pinned Linux library/integration and seven-job release gate; not all150 LangArena, production, other platforms or speed acceptance.'}
File.write(File.join(out,'verification.json'),JSON.pretty_generate(result))
puts JSON.generate({commit:commit,passed:passed.size,ignored:ignored.size,jobs:jobs.size})
