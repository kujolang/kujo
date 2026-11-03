require 'json'
require 'tmpdir'
require 'open3'
data = {'schema'=>'kujo.owner-concurrent-resource/v1', 'lineages'=>8, 'rounds_per_lineage'=>16,
        'baseline_rss_bytes'=>1000, 'baseline_executable_mapping_bytes'=>500, 'observations'=>[]}
8.times do |lineage|
  16.times do |round|
    data['observations'] << {'lineage'=>lineage, 'round'=>round, 'active_owners'=>2, 'idle_owners'=>2,
      'active_rss_bytes'=>1200, 'idle_rss_bytes'=>1200,
      'active_executable_mapping_bytes'=>600, 'idle_executable_mapping_bytes'=>600}
  end
  data['observations'] << {'lineage'=>lineage, 'after_drop_rss_bytes'=>1100, 'after_drop_executable_mapping_bytes'=>500}
end
results = []
Dir.mktmpdir('owner-concurrent-verifier') do |directory|
  %w[valid duplicate-round missing-drop wrong-owner bad-rss wrong-test failed-test].each do |mode|
    sample = Marshal.load(Marshal.dump(data))
    sample['observations'][1]['round'] = 0 if mode == 'duplicate-round'
    sample['observations'].pop if mode == 'missing-drop'
    sample['observations'][0]['idle_owners'] = 1 if mode == 'wrong-owner'
    sample['observations'][0]['idle_rss_bytes'] = -1 if mode == 'bad-rss'
    name = mode == 'wrong-test' ? 'fabricated' : 'vm::task_owner_resource_tests::task_owner_resource_diagnostic'
    summary = mode == 'failed-test' ? 'FAILED. 0 passed; 1 failed;' : 'ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1341 filtered out;'
    log = directory + '/sample.log'
    File.write(log, "running 1 test\ntest #{name} ... OWNER_CONCURRENT_RESOURCE #{JSON.generate(sample)}\nok\ntest result: #{summary}\n")
    out, err, status = Open3.capture3('ruby', File.join(__dir__, 'verify-owner-concurrent-resources.rb'), log)
    raise "unexpected #{mode}: #{out} #{err}" unless status.success? == (mode == 'valid')
    results << {control: mode, accepted: status.success?}
  end
end
puts JSON.pretty_generate(results)
