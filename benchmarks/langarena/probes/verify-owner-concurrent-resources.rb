require 'json'
require 'digest'
raise 'usage: LOG' unless ARGV.size == 1
raw = File.read(ARGV.first)
lines = raw.scan(/OWNER_CONCURRENT_RESOURCE (\{[^\n]+\})/).flatten
raise 'record count' unless lines.size == 1
data = JSON.parse(lines.first)
raise 'schema' unless data['schema'] == 'kujo.owner-concurrent-resource/v1'
raise 'coverage' unless data['lineages'] == 8 && data['rounds_per_lineage'] == 16
raise 'test outcome' unless raw.scan(/test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured;/).size == 1
raise 'test membership' unless raw.scan(/^test (.+) \.\.\. /).flatten == ['vm::task_owner_resource_tests::task_owner_resource_diagnostic']
%w[baseline_rss_bytes baseline_executable_mapping_bytes].each do |key|
  raise 'baseline' unless data[key].is_a?(Integer) && data[key] > 0
end
observations = data.fetch('observations')
raise 'observation count' unless observations.size == 136
expected = (0...8).flat_map { |lineage| (0...16).map { |round| [lineage, round] } }
active = observations.select { |row| row.key?('round') }
raise 'round membership' unless active.map { |row| row.values_at('lineage', 'round') } == expected
active.each do |row|
  raise 'owners' unless row.values_at('active_owners', 'idle_owners') == [2, 2]
  %w[active_rss_bytes active_executable_mapping_bytes idle_rss_bytes idle_executable_mapping_bytes].each do |key|
    raise 'observation' unless row[key].is_a?(Integer) && row[key] > 0
  end
end
drops = observations.reject { |row| row.key?('round') }
raise 'drop membership' unless drops.map { |row| row['lineage'] } == (0...8).to_a
drops.each do |row|
  %w[after_drop_rss_bytes after_drop_executable_mapping_bytes].each do |key|
    raise 'drop observation' unless row[key].is_a?(Integer) && row[key] > 0
  end
end
puts JSON.pretty_generate({status: 'observations-verified-review-pending', log_sha256: Digest::SHA256.hexdigest(raw),
  max_idle_rss_delta: active.map { |r| r['idle_rss_bytes'] - data['baseline_rss_bytes'] }.max,
  max_idle_executable_delta: active.map { |r| r['idle_executable_mapping_bytes'] - data['baseline_executable_mapping_bytes'] }.max,
  lineage_drop_deltas: drops.map { |r| {lineage: r['lineage'], rss: r['after_drop_rss_bytes'] - data['baseline_rss_bytes'], executable: r['after_drop_executable_mapping_bytes'] - data['baseline_executable_mapping_bytes']} },
  scope: 'Record integrity only; inspect retention and reclamation before acceptance'})
