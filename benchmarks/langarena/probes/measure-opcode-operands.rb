# Repository-maintenance collector for the Rust metadata extraction probe.
require 'json'
require 'csv'
require 'digest'
require 'open3'

binary, library, output = ARGV
raise 'usage: BINARY LIBRARY OUTPUT.json' unless output
raise 'output already exists' if File.exist?(output)
source = 'benchmarks/langarena/probes/opcode_operand_cost.rs'
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
identities = [binary, library, source].to_h { |path| [path, sha.call(path)] }
stdout, stderr, status = Open3.capture3(binary)
raise "probe failed: #{stderr}" unless status.success?
rows = CSV.parse(stdout, headers: true).map(&:to_h)
expected = (0...2_000_000).sum { |n| n % 256 }
raise 'row count' unless rows.length == 16
8.times do |round|
  pair = rows.slice(round * 2, 2)
  order = round.even? ? %w[owned borrowed] : %w[borrowed owned]
  raise 'order' unless pair.map { |row| row['path'] } == order
  pair.each do |row|
    raise 'round' unless Integer(row['round']) == round
    raise 'reads' unless Integer(row['reads']) == 2_000_000
    raise 'checksum' unless Integer(row['checksum']) == expected
    elapsed = Float(row['ns_per_read'])
    raise 'elapsed' unless elapsed.finite? && elapsed > 0
  end
end
summary = %w[owned borrowed].to_h do |path|
  ns = rows.select { |row| row['path'] == path }.map { |row| Float(row['ns_per_read']) }.sort
  [path, {min: ns.first, median: (ns[3] + ns[4]) / 2, max: ns.last}]
end
identities.each { |path, hash| raise 'artifact drift' unless sha.call(path) == hash }
File.write(output, JSON.pretty_generate({status: 'verified-helper-only', identities: identities,
  stdout: stdout, stderr: stderr, exit: status.exitstatus, summary_ns: summary,
  limitations: 'Opaque owned-enum extraction versus borrowed extraction. This intentionally prevents clone elision; it does not measure actual VM dispatch, value cloning, workload speedup or clean-host performance.'}) + "\n")
puts JSON.generate(summary)
