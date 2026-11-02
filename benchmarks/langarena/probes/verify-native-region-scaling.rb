# Verify complete diagnostic coverage and raw timers before summarizing pairs.
require 'json'
require 'digest'
root = File.expand_path(ARGV.fetch(0))
report = JSON.parse(File.read(root + '/results.json'))
raise 'measurement incomplete' unless report['status'] == 'measured-review-pending'
raise 'protocol mismatch' unless report['rounds'] == 7 && report['groups'] == 32768
files = report.fetch('files')
files.each { |path, hash| raise "identity drift: #{path}" unless Digest::SHA256.file(path).hexdigest == hash }
source = 'benchmarks/langarena/probes/native-region-scaling.kujo'
raise 'unbound probe' unless files.key?(source)
rows = report.fetch('rows')
raise 'sample count' unless rows.size == 504
expected = %w[integer-array negative-array float-array bytes integer-dictionary string-dictionary].product([8, 256, 8192], %w[vm jit])
summary = []
cursor = 0
binaries = {}
expected.each do |name, width, runtime|
  pairs = []
  samples = {'baseline' => [], 'candidate' => []}
  7.times do |round|
    variants = round.even? ? %w[baseline candidate] : %w[candidate baseline]
    pair = {}
    variants.each do |variant|
      row = rows.fetch(cursor)
      raise 'coverage/order' unless row.values_at('case', 'width', 'runtime', 'round', 'variant') == [name, width, runtime, round + 1, variant]
      raise 'failed sample' unless row['passed'] == true && row['exit'] == 0 && row['signal'].nil? && row['timed_out'] == false
      argv = row.fetch('argv')
      binary = binaries[variant] ||= argv.first
      expected_argv = [binary, 'run'] + (runtime == 'jit' ? ['--jit'] : []) + [source, name, width.to_s, '32768', 'measure']
      raise 'invocation mismatch' unless argv == expected_argv && files.key?(binary)
      stdout = File.expand_path(row.fetch('stdout'))
      stderr = File.expand_path(row.fetch('stderr'))
      raise 'output mapping' unless stdout == "#{root}/#{cursor}.out" && stderr == "#{root}/#{cursor}.err"
      lines = File.read(stdout).lines.map(&:strip)
      raise 'unexpected output' unless lines.size == 2 && lines[0].match?(/\A@region_ns [0-9]+\z/) && lines[1] == 'native-region-scaling: OK'
      # CLI opt-in diagnoses the top-level MakeClosure even when the called
      # loop function is eligible for native compilation. Preserve that exact
      # notice; reject any additional or different diagnostic.
      expected_stderr = runtime == 'jit' ? "JIT opt-in requested; unsupported bytecode regions will use the VM (unsupported opcode in chunk '<script>' at instruction 0: MakeClosure(0)).\n" : ''
      raise 'unexpected stderr' unless File.read(stderr) == expected_stderr
      nanoseconds = Integer(lines[0].split.last)
      raise 'timer mismatch' unless row['kernel_ns'] == nanoseconds && nanoseconds > 0
      samples[variant] << nanoseconds
      pair[variant] = nanoseconds
      cursor += 1
    end
    pairs << pair.fetch('baseline').to_f / pair.fetch('candidate')
  end
  summary << {case: name, width: width, runtime: runtime,
              median_paired_speedup: pairs.sort[3], paired_speedups: pairs,
              candidate_faster_pairs: pairs.count { |ratio| ratio > 1 },
              kernel_ns: samples.transform_values { |values| {median: values.sort[3], range: values.minmax} }}
end
raise 'same executable used twice' if File.realpath(binaries.fetch('baseline')) == File.realpath(binaries.fetch('candidate'))
output = {status: 'passed', samples: cursor, groups: 32768, reads_per_group: 8,
          scope: 'Local synthetic diagnostic only; no independent-session or clean-host claim',
          stderr_policy: '252 JIT opt-in samples retain the exact top-level MakeClosure fallback notice; VM stderr is empty. This does not prove native kernel participation.',
          report_sha256: Digest::SHA256.file(root + '/results.json').hexdigest,
          verifier_sha256: Digest::SHA256.file(__FILE__).hexdigest,
          results: summary}
File.write(root + '/verified.json', JSON.pretty_generate(output) + "\n")
puts "Verified #{cursor} samples across #{summary.size} case/width/runtime cells."
