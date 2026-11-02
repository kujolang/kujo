# Repository-maintenance evidence packaging, not measured language behavior.
require 'json'
require 'digest'
require 'base64'
require 'zlib'

root = File.expand_path(ARGV.fetch(0))
output = ARGV.fetch(1)
raise 'output already exists' if File.exist?(output)
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
state = JSON.parse(File.read(root + '/state.json'))
report = JSON.parse(File.read(root + '/samples/report.json'))
raise 'pipeline not terminal-success' unless state['status'] == 'profiles-collected-review-pending'
raise 'failed pipeline command' unless state['commands'].size == 3 && state['commands'].all? { |c| c['exit'] == 0 && c['stop'].nil? && c['signal'].nil? }
state.fetch('source_files').each { |path, hash| raise "source drift: #{path}" unless sha.call(path) == hash }
report.fetch('sources').each { |path, hash| raise "input drift: #{path}" unless sha.call(path) == hash }
raise 'harness drift' unless sha.call(report['binary']) == report['binary_sha256'] && report['binary_sha256'] == state['harness_sha256']
raise 'library drift' unless sha.call('target/release/libkujo.rlib') == state['library_sha256']
cases = JSON.parse(File.read('benchmarks/langarena/test.json')).to_h { |c| [c['name'], c['checksum']] }
expected = %w[Sort::Quick Sort::Merge Etc::NeuralNet].product(%w[vm jit]).sort
raise 'coverage' unless report['status'] == 'measured' && report['rows'].map { |r| [r['name'], r['mode']] }.sort == expected
report['rows'].each do |row|
  lines = row['stdout'].lines.map(&:strip)
  raise 'checksum or execution failure' unless row['exit'] == 0 && row['stderr'].empty? && lines.include?("#{row['name']}: OK #{cases.fetch(row['name'])}") && lines.include?('Summary: 1, 1, 0')
  raise 'sampler failure' unless row['sample']['exit'] == 0 && row['sample']['trigger_line'].start_with?('@profile warmup-done ')
  raise 'sample drift' unless sha.call(root + '/samples/' + row['sample_file']) == row['sample_sha256']
end
raise 'attribution failed' unless system('ruby', 'benchmarks/langarena/probes/attribute-kernel-stacks.rb', root)
paths = %w[state.json baseline-pipeline.rb toolchain.txt accepted-library.log symbolized-harness.log samples.log samples/report.json stack-attribution.json].map { |p| root + '/' + p }
paths += Dir[root + '/samples/*.sample.txt']
paths += %w[benchmarks/langarena/probes/attribute-kernel-stacks.rb benchmarks/langarena/probes/archive-native-baseline.rb benchmarks/langarena/probes/sample-symbolized-kernels.rb benchmarks/langarena/probes/kernel_profile.rs benchmarks/langarena/test.json]
files = paths.to_h do |path|
  bytes = File.binread(path)
  key = path.start_with?(root + '/') ? path.delete_prefix(root + '/') : path
  [key, {sha256: Digest::SHA256.hexdigest(bytes), base64: Base64.strict_encode64(bytes)}]
end
Zlib::GzipWriter.open(output) { |gz| gz.write(JSON.generate({schema: 1, purpose: 'Accepted runtime sampled-stack baseline; not a speed comparison', files: files})) }
archive = JSON.parse(Zlib::GzipReader.open(output, &:read))
archive['files'].each { |path, item| raise "archive corruption: #{path}" unless Digest::SHA256.hexdigest(Base64.strict_decode64(item['base64'])) == item['sha256'] }
puts "Verified #{state['source_files'].size} source hashes, six checksums/samples, and #{files.size} archived files."
