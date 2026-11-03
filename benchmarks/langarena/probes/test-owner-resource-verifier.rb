# Synthetic parser controls only; never production/resource observations.
require 'json'
require 'digest'
require 'tmpdir'
require 'open3'
verifier = File.expand_path('verify-owner-resources.rb', __dir__)
head, error, status = Open3.capture3('git', 'rev-parse', 'HEAD')
raise error unless status.success?
controls = {
  'valid' => nil, 'archive-valid' => nil,
  'archive-build-failed' => 'archive build outcome', 'archive-wrong-binary' => 'archive build binary', 'missing-case' => 'case coverage',
  'duplicate-case' => 'case coverage', 'timeout' => 'process outcome',
  'changed-log' => 'log identity', 'wrong-environment' => 'environment',
  'wrong-invocation' => 'invocation', 'missing-checkpoint' => 'checkpoint coverage',
  'recompilation' => 'compile control', 'lost-native-entry' => 'native participation',
  'unpooled-owner' => 'unpooled retained owner', 'lost-pooled-owner' => 'same-code pool not retained',
  'invalid-memory' => 'rss_bytes', 'duplicate-receipt' => 'receipt count',
  'wrong-source' => 'source identity', 'changed-binary' => 'binary changed'
}
results = []
Dir.mktmpdir('kujo-resource-verifier-') do |dir|
  binary = File.join(dir, 'synthetic-not-executable')
  controls.each do |control, expected_error|
    File.write(binary, 'synthetic binary identity fixture')
    File.write(dir+'/source.txt', control == 'wrong-source' ? '0'*40+"\n" : head)
    File.write(dir+'/executable.json', JSON.generate({path: binary, sha256: Digest::SHA256.file(binary).hexdigest}))
    build = [
      {reason: 'compiler-artifact', profile: {test: true}, target: {name: 'kujo'}, executable: control == 'archive-wrong-binary' ? '/wrong/binary' : binary},
      {reason: 'build-finished', success: control != 'archive-build-failed'}
    ]
    File.write(dir+'/build.jsonl', build.map { |row| JSON.generate(row) }.join("\n")+"\n")

    cases = %w[pooled unpooled].product(%w[same distinct], [32, 1024]).map do |mode, shape, count|
      target = mode == 'pooled' && shape == 'same' && count == 32
      unpooled = mode == 'unpooled' && shape == 'same' && count == 32
      observations = (32..count).step(32).map do |tasks|
        {tasks: tasks, rss_bytes: 4096, executable_mapping_bytes: 4096, idle_owners: mode == 'pooled' ? 1 : 0}
      end
      data = {schema: 'kujo.owner-resource/v1', mode: mode, shape: shape, count: count,
              baseline_rss_bytes: 4096, baseline_executable_mapping_bytes: 4096,
              after_drop_rss_bytes: 4096, after_drop_executable_mapping_bytes: 4096,
              observations: observations,
              measurements: {schema: 'kujo.runtime-measurements/v1', scope: 'process', outcome: 'success',
                counters: {jit_compile_entries: mode == 'pooled' && shape == 'same' ? 1 : count,
                           jit_native_boundary_entries: count}}}
      observations.pop if target && control == 'missing-checkpoint'
      observations.first[:rss_bytes] = -1 if target && control == 'invalid-memory'
      observations.first[:idle_owners] = 0 if target && control == 'lost-pooled-owner'
      observations.first[:idle_owners] = 1 if unpooled && control == 'unpooled-owner'
      data[:measurements][:counters][:jit_compile_entries] = count if target && control == 'recompilation'
      data[:measurements][:counters][:jit_native_boundary_entries] = 0 if target && control == 'lost-native-entry'
      receipt = 'OWNER_RESOURCE '+JSON.generate(data)+"\n"
      raw = receipt + "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out\n"
      raw += receipt if target && control == 'duplicate-receipt'
      log = dir+"/#{mode}-#{shape}-#{count}.log"
      File.write(log, raw)
      row = {mode: mode, shape: shape, count: count, exit: 0, signal: nil,
             log_sha256: Digest::SHA256.hexdigest(raw),
             environment: {'KUJO_OWNER_RESOURCE_MODE'=>mode, 'KUJO_OWNER_RESOURCE_SHAPE'=>shape, 'KUJO_OWNER_RESOURCE_COUNT'=>count.to_s},
             argv: ['timeout','180',binary,'--exact','vm::task_owner_resource_tests::task_owner_resource_diagnostic','--ignored','--nocapture','--test-threads=1']}
      row[:exit] = 124 if target && control == 'timeout'
      row[:environment]['KUJO_OWNER_RESOURCE_COUNT'] = '1' if target && control == 'wrong-environment'
      row[:argv][1] = '999' if target && control == 'wrong-invocation'
      File.write(log, raw+'changed') if target && control == 'changed-log'
      row
    end
    cases.pop if control == 'missing-case'
    cases[-1] = cases.first if control == 'duplicate-case'
    File.write(dir+'/cases.json', JSON.generate(cases))
    File.write(binary, 'changed identity') if control == 'changed-binary'
    archived = control.start_with?('archive-')
    acceptance_file = dir+(archived ? '/archived-verified.json' : '/verified.json')
    File.delete(acceptance_file) if File.exist?(acceptance_file)
    argv = ['ruby', verifier, dir]
    argv += ['archive', head.strip] if archived
    out, err, result = Open3.capture3(*argv)
    if expected_error
      raise "#{control} accepted or wrong rejection: #{out} #{err}" unless !result.success? && err.include?(expected_error)
      raise "#{control} wrote acceptance" if File.exist?(acceptance_file)
    else
      raise "valid rejected: #{out} #{err}" unless result.success?
      verified = JSON.parse(File.read(acceptance_file))
      raise 'archive binary scope' if archived && !verified['input_identity_scope'].include?('executable bytes unavailable')
      raise 'incorrect review scope' unless verified['status'] == 'verified-observations-review-pending' && verified['rows'].size == 8
    end
    results << {control: control, expected_acceptance: expected_error.nil?, exit: result.exitstatus}
  end
end
puts JSON.pretty_generate({scope: 'Synthetic verifier controls; no runtime resource evidence', controls: results})
