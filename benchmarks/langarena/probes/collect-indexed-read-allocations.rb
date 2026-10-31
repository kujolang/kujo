# Collect warmed helper counts; this is not a benchmark score or native coverage gate.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
binary, library, output = ARGV
abort 'BINARY LIBRARY NEW_OUTPUT_DIRECTORY' unless ARGV.size == 3
abort 'output must be new' if File.exist?(output)
FileUtils.mkdir_p(output)
sha = ->(p) { Digest::SHA256.file(p).hexdigest }
source = File.join(__dir__, 'native_indexed_read_allocations.rs')
paths = [binary, library, source, __FILE__].map { |p| File.realpath(p) }
identities = paths.to_h { |p| [p, sha.call(p)] }
rows = []
3.times do |round|
  stdout, stderr, status = Open3.capture3(File.realpath(binary))
  File.write(File.join(output, "round-#{round}.stdout"), stdout)
  File.write(File.join(output, "round-#{round}.stderr"), stderr)
  abort stderr unless status.success?
  parsed = stdout.lines.map do |line|
    m = line.match(/\A(\w+) reads=(\d+) requests=(\d+) requested_bytes=(\d+) diagnostic_elapsed_ns=(\d+)\s*\z/)
    abort "unexpected row: #{line}" unless m
    {shape: m[1], reads: m[2].to_i, requests: m[3].to_i, bytes: m[4].to_i, diagnostic_elapsed_ns: m[5].to_i}
  end
  abort 'coverage mismatch' unless parsed.map { |r| r[:shape] } == %w[integer_array float_array float_dictionary owned_array]
  abort 'read count mismatch' unless parsed.all? { |r| r[:reads] == 100000 }
  rows << {round: round, exit: status.exitstatus, results: parsed}
end
identities.each { |p, h| abort "artifact changed: #{p}" unless sha.call(p) == h }
report = {status: 'collected', identities: identities, rounds: rows,
          limits: ['Direct helper calls, no generated-code execution proof.',
                   'Root setup and 100 warmup reads excluded; compilation absent.',
                   'Requested allocation traffic, not CPU cost, retained heap or comparative speed.',
                   'Owned-result checks cover tag and successful release, not deep payload equality.']}
File.write(File.join(output, 'report.json'), JSON.pretty_generate(report) + "\n")
puts JSON.generate(report)
