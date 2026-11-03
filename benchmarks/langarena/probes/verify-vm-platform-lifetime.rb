# Verify downloaded targeted lifetime artifacts; workflow/job identity is reviewed separately.
require 'json'
require 'digest'
require 'open3'
raise 'usage: ARTIFACT_DIRECTORY PLATFORM EXPECTED_COMMIT' unless ARGV.size == 3
out, platform, commit = ARGV
hosts = {'linux-x64'=>'x86_64-unknown-linux-gnu','linux-arm64'=>'aarch64-unknown-linux-gnu',
         'macos-x64'=>'x86_64-apple-darwin','macos-arm64'=>'aarch64-apple-darwin',
         'windows-x64'=>'x86_64-pc-windows-msvc'}
host = hosts.fetch(platform)
raise 'full commit required' unless commit.match?(/\A[0-9a-f]{40}\z/)
files = %w[lifetime-source.txt lifetime-library.log lifetime-integration.log]
raw = files.to_h { |f| [f,File.binread(File.join(out,f))] }
source = raw.fetch('lifetime-source.txt').gsub("\r\n","\n")
raise 'source/toolchain identity' unless source.lines.first.strip == commit && source.lines.map(&:strip).include?('host: '+host)
library = raw.fetch('lifetime-library.log').gsub("\r\n","\n")
integration = raw.fetch('lifetime-integration.log').gsub("\r\n","\n")
expected_library = %w[disabling_jit_does_not_destroy_cached_code returned_values_functions_and_snapshots_survive_original_vm]
vm_source, vm_error, vm_status = Open3.capture3('git', 'show', "#{commit}:src/vm.rs")
raise "missing pinned VM source: #{vm_error}" unless vm_status.success?
%w[task_code_reuse_obeys_each_invocations_capability_policy
   task_code_reuse_recovers_cancellation_errors_and_rebinding
   task_code_reuse_keeps_globals_fresh_and_compilation_flat
   idle_code_pool_bounds_retention_and_exclusively_moves_entries
   exclusive_code_owner_survives_vm_and_worker_transfer
   transferred_code_uses_current_globals_and_keeps_literal_owners
   transferred_code_observes_current_cancellation_token].each do |name|
  expected_library << name if vm_source.include?("fn #{name}()")
end
expected_library << 'vm_drop_revokes_its_executable_entry' unless platform == 'windows-x64'
expected_library.map! { |name| 'vm::code_owner_lifetime_tests::'+name }
raise 'owner-test membership' unless library.scan(/^test (.+) \.\.\. ok$/).flatten.sort == expected_library.sort
summaries = lambda do |text|
  text.scan(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/).map { |row| row.map(&:to_i) }
end
lib_summary = summaries.call(library)
raise 'library result' unless lib_summary.size == 1 && lib_summary[0][0] == expected_library.size && lib_summary[0][1,3] == [0,0,0]
expected_targets = %w[generator_continuation_contracts imported_vm_callback callback_module_cache
  jit_callback_depth language_task_contracts jit_cold_callback task_jit_policy
  task_module_snapshot_contracts task_global_snapshot_contracts imported_task_globals_contracts]
results = summaries.call(integration)
raise 'integration coverage/results' unless results.size == 10 && results.all? { |row| row[1..-1] == [0,0,0,0] }
names = integration.scan(/^test (.+) \.\.\. ok$/).flatten
# These ten suites use top-level, directly annotated tests. Pin membership to
# the tested commit rather than accepting any sixty successful log lines.
# This is deliberately not a general Rust test-discovery parser.
test_sources = expected_targets.to_h do |target|
  path = "tests/#{target}.rs"
  bytes, error, status = Open3.capture3('git', 'show', "#{commit}:#{path}")
  raise "missing pinned test source #{path}: #{error}" unless status.success?
  tests = bytes.scan(/^#\[test\]\s*\nfn ([a-zA-Z0-9_]+)\(/).flatten
  raise "unsupported test declarations in #{path}" unless tests.size == bytes.scan(/^#\[test\]/).size && !tests.empty?
  [target, {source_sha256: Digest::SHA256.hexdigest(bytes), tests: tests.sort}]
end
expected_names = test_sources.values.flat_map { |entry| entry[:tests] }.sort
raise 'pinned integration membership' unless names.sort == expected_names && results.sum { |row| row[0] } == expected_names.size
messages = integration.lines.select { |line| line.start_with?('{') }.map { |line| JSON.parse(line) }
targets = messages.select { |m| m['reason'] == 'compiler-artifact' && m.dig('profile','test') }.map { |m| m.dig('target','name') }
raise 'integration binaries' unless targets.sort == expected_targets.sort
raise 'build result' unless messages.select { |m| m['reason'] == 'build-finished' }.map { |m| m['success'] } == [true]
workflow, workflow_error, workflow_status = Open3.capture3('git', 'show', "#{commit}:.github/workflows/vm-code-lifetime.yml")
raise "missing pinned workflow: #{workflow_error}" unless workflow_status.success?
no_jit_passed = 0
accounting_passed = 0
if platform == 'linux-x64' && workflow.include?('name: Test owner boundary without JIT')
  raw['lifetime-no-jit.log'] = File.binread(File.join(out, 'lifetime-no-jit.log'))
  no_jit = raw.fetch('lifetime-no-jit.log').gsub("\r\n", "\n")
  expected_no_jit = ['vm::code_owner_lifetime_tests::returned_values_functions_and_snapshots_survive_original_vm']
  expected_no_jit << 'interpreter::native_functions::web_data::boundary_tests::disabled_network_web_helpers_report_feature_error' if workflow.include?('--lib disabled_network_web_helpers_report_feature_error')
  raise 'no-JIT owner membership' unless no_jit.scan(/^test (.+) \.\.\. ok$/).flatten.sort == expected_no_jit.sort
  checks = summaries.call(no_jit)
  raise 'no-JIT owner result' unless checks.size == expected_no_jit.size && checks.all? { |row| row[0,4] == [1,0,0,0] }
  no_jit_passed = expected_no_jit.size
end
if platform == 'linux-x64' && workflow.include?('name: Test native owner accounting')
  raw['lifetime-accounting.log'] = File.binread(File.join(out, 'lifetime-accounting.log'))
  accounting = raw.fetch('lifetime-accounting.log').gsub("\r\n", "\n")
  expected = %w[jit::tests::owner_accounting_covers_each_native_definition_path jit::tests::loop_statistics_include_reuse_and_mutated_code_identity]
  expected << 'jit::tests::owner_metadata_accounts_literals_capacity_and_unknown_payloads' if workflow.include?('--lib owner_metadata_accounts_literals_capacity_and_unknown_payloads')
  expected << 'jit::tests::owner_reset_rejects_incomplete_or_incompatible_compilers' if workflow.include?('--lib owner_reset_rejects_incomplete_or_incompatible_compilers')
  raise 'accounting membership' unless accounting.scan(/^test (.+) \.\.\. ok$/).flatten.sort == expected.sort
  checks = summaries.call(accounting)
  raise 'accounting results' unless checks.size == expected.size && checks.all? { |row| row[0,4] == [1,0,0,0] }
  accounting_passed = expected.size
end
result = {status:'verified',platform:platform,commit:commit,toolchain:source,
          library_tests:expected_library,library_filtered:lib_summary[0][4],integration_targets:targets,
          integration_passed:expected_names.size,total_passed:expected_names.size+expected_library.size+no_jit_passed+accounting_passed,
          accounting_passed:accounting_passed,
          no_jit_passed:no_jit_passed,vm_source_sha256:Digest::SHA256.hexdigest(vm_source),
          integration_test_sources:test_sources,
          executable_revocation_test:platform != 'windows-x64',
          files:raw.to_h { |name,bytes| [name,Digest::SHA256.hexdigest(bytes)] },
          scope:'Targeted lifetime/semantic verification only; Windows reclamation not proved, remaining library tests excluded, workflow/job provenance reviewed separately.'}
File.write(File.join(out,'verification.json'),JSON.pretty_generate(result))
puts JSON.generate({platform:platform,total_passed:result[:total_passed],executable_revocation_test:result[:executable_revocation_test]})
