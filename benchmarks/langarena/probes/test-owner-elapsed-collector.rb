# Exercise fail-closed collection and paired scheduling with controlled executables.
require 'tmpdir'
require 'json'
require 'open3'
collector = File.join(__dir__, 'compare-owner-elapsed.rb')
results = []
Dir.mktmpdir('owner-elapsed-controls') do |directory|
  good = directory + '/good'
  bad = directory + '/bad'
  File.write(good, "#!/bin/sh\ncase \" $* \" in *' --jit '*) printf 'JIT opt-in requested; unsupported bytecode regions will use the VM (unsupported opcode in chunk).\\n' >&2;; esac\nprintf 'async-worker-economics: OK\\n'\n")
  File.write(bad, "#!/bin/sh\nprintf 'wrong checksum\\n'\n")
  File.chmod(0755, good, bad)
  output, error, status = Open3.capture3('ruby', collector, good, good, directory + '/valid')
  raise error unless status.success?
  report = JSON.parse(File.read(directory + '/valid/report.json'))
  raise 'coverage' unless report['rows'].size == 192 && report['rows'].count { |r| r['warmup'] } == 24 && report['comparisons'].size == 12
  raise 'instrumentation' if report['rows'].any? { |r| r['argv'].include?('--measurements') }
  report['rows'].each_slice(2) do |pair|
    raise 'paired cases' unless pair.map { |r| r.values_at('round', 'runtime', 'mode', 'workers') }.uniq.size == 1
    raise 'paired variants' unless pair.map { |r| r['variant'] }.sort == %w[baseline candidate]
  end
  raise 'order variation' unless report['rows'].each_slice(2).map { |pair| pair.first['variant'] }.uniq.size == 2
  results << '192 samples, 24 excluded warmups, twelve paired cells, no measurement flags'
  _, _, status = Open3.capture3('ruby', collector, good, bad, directory + '/invalid')
  report = JSON.parse(File.read(directory + '/invalid/report.json'))
  raise 'wrong output accepted' if status.success? || report['status'] != 'failed' || report.key?('comparisons')
  results << 'incorrect output rejects comparison'
  _, _, status = Open3.capture3({'DEBUG_JIT' => '1'}, 'ruby', collector, good, good, directory + '/override')
  raise 'instrumented environment accepted' if status.success? || File.exist?(directory + '/override')
  results << 'diagnostic environment rejected before execution'
end
puts JSON.pretty_generate(results)
