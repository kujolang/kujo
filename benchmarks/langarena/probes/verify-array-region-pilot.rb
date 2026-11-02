# Screening evidence only. No extrapolated clean-host speeds or significance claim.
require 'json'
require 'digest'
root = File.expand_path(ARGV.fetch(0))
identity = JSON.parse(File.read(root + '/region-pilot-identities.json'))
rounds = identity.fetch('rounds')
raise 'invalid round count' unless rounds.is_a?(Integer) && rounds > 0 && rounds.odd? && rounds <= 99
identity.fetch('files').each { |path, hash| raise "identity drift: #{path}" unless Digest::SHA256.file(path).hexdigest == hash }
languages = ['Baseline VM', 'Baseline JIT', 'Latest VM', 'Latest JIT', 'Rust', 'Goose']
config = JSON.parse(File.read('benchmarks/langarena/test.json'))
summary = []
def median(values)
  values.sort.fetch(values.size / 2)
end
{'quick' => 'Sort::Quick', 'merge' => 'Sort::Merge', 'neural' => 'Etc::NeuralNet'}.each do |slug, name|
  directory = root + '/region-pilot-' + slug
  manifest = JSON.parse(File.read(directory + '/samples.json'))
  raise 'rounds' unless manifest['rounds'] == rounds && manifest['result_files'].size == rounds * 6
  raise 'duplicate evidence' unless manifest['result_files'].uniq.size == rounds * 6
  expected_binaries = {
    'Baseline VM' => '.audit-evidence/checkpoint-2026-11-02/frozen/checkpoint-kujo',
    'Baseline JIT' => '.audit-evidence/checkpoint-2026-11-02/frozen/checkpoint-kujo',
    'Latest VM' => identity.fetch('candidate_binary', 'target/release/kujo'),
    'Latest JIT' => identity.fetch('candidate_binary', 'target/release/kujo'),
    'Rust' => '.audit-evidence/checkpoint-2026-11-02/frozen/rust-benchmarks',
    'Goose' => '.audit-evidence/checkpoint-2026-11-02/frozen/goose-benchmarks'
  }
  raise 'unbound candidate binary' unless identity.fetch('files').key?(expected_binaries.fetch('Latest JIT'))
  raise 'binary coverage' unless manifest['binaries'].keys.sort == languages.sort
  expected_binaries.each { |language, path| raise 'binary mapping' unless File.realpath(manifest['binaries'][language]) == File.realpath(path) }
  raise 'resource platform' unless manifest['resource_measurement'] && manifest['resource_platform'] == 'Darwin'
  {'configuration_sha256' => 'config.json', 'timed_port_sha256' => 'timed-port.kujo'}.each do |key, file|
    raise 'input hash' unless Digest::SHA256.file(directory + '/' + file).hexdigest == manifest[key]
  end
  raise 'original port' unless manifest['original_port_sha256'] == identity['files']['benchmarks/langarena/port.kujo']
  raise 'configuration' unless manifest['configuration_sha256'] == identity['files']['benchmarks/langarena/test.json']
  expected_config = config.find { |row| row['name'] == name }
  rows = manifest['result_files'].map { |file| JSON.parse(File.read(directory + '/' + file)) }
  rows.each_with_index do |row, index|
    round = index / 6 + 1
    language = languages[(round - 1 + index % 6) % 6]
    raise 'order/coverage' unless row['round'] == round && row['language'] == language && row['benchmark'] == name
    raise 'case config' unless row['config'] == expected_config
    raise 'failure' unless row['passed'] && row['exitcode'] == 0 && !row['timed_out'] && !row['cancelled'] && !row['stdout_truncated'] && !row['stderr_truncated']
    raise 'missing RSS' unless row['peak_rss_kib'].is_a?(Numeric) && row['peak_rss_kib'] > 0
    stdout = File.read(directory + '/' + row['stdout_path'])
    stderr = File.read(directory + '/' + row['stderr_path'])
    raise 'missing resource receipt' unless stderr.include?('maximum resident set size')
    rss = stderr.lines.find { |line| line.include?('maximum resident set size') }.split.first.to_f / 1024
    raise 'RSS mismatch' unless rss == row['peak_rss_kib']
    if language == 'Rust' || language == 'Goose'
      line = stdout.lines.find { |line| line.start_with?(name + ': OK in ') }
      raise 'comparator outcome' unless line && stdout.include?(', 1, 1, 0')
      seconds = Float(line.split(' in ').last.strip.delete_suffix('s'))
    else
      lines = stdout.lines.map(&:strip)
      raise 'checksum' unless lines.count("#{name}: OK #{expected_config['checksum']}") == 1 && lines.count('Summary: 1, 1, 0') == 1
      timing = lines.select { |line| line.start_with?('@comparison_ns ') }
      raise 'timer count' unless timing.size == 1
      seconds = Float(timing.first.split.last) / 1_000_000_000
    end
    raise 'timer mismatch' unless seconds == row['measured_seconds'] && seconds.finite? && seconds > 0
  end
  modes = %w[VM JIT].to_h do |mode|
    ratios = (1..rounds).map do |round|
      before = rows.find { |r| r['round'] == round && r['language'] == "Baseline #{mode}" }
      after = rows.find { |r| r['round'] == round && r['language'] == "Latest #{mode}" }
      before['measured_seconds'] / after['measured_seconds']
    end
    [mode, {paired_speedups: ratios, median_paired_speedup: median(ratios)}]
  end
  timings = languages.to_h do |language|
    samples = rows.select { |row| row['language'] == language }
    [language, {measured_seconds_median: median(samples.map { |r| r['measured_seconds'] }), measured_seconds_range: samples.map { |r| r['measured_seconds'] }.minmax, peak_rss_kib_median: median(samples.map { |r| r['peak_rss_kib'] })}]
  end
  summary << {benchmark: name, pairs: modes, variants: timings}
end
File.write(root + '/region-pilot-verified.json', JSON.pretty_generate({status: 'passed', samples: rounds * 18, scope: "#{rounds}-round local screening; no statistical significance, independent-session or clean-host claim; competitors have millisecond-rounded loop times", results: summary}) + "\n")
puts JSON.pretty_generate(summary.map { |row| {benchmark: row[:benchmark], pairs: row[:pairs]} })
