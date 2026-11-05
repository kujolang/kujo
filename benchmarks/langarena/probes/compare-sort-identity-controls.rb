# Two byte-identical baselines plus candidate; all six orders twice, VM only.
require 'json'
require 'digest'
require 'fileutils'
raise 'usage: FROZEN_ARTIFACT NEW_OUTPUT [WORKLOAD...]' unless ARGV.size >= 2
frozen, out = ARGV.first(2).map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
raise 'diagnostic environment' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
binaries = {'baseline_a' => frozen + '/baseline-kujo', 'baseline_b' => frozen + '/baseline-copy-kujo', 'candidate' => frozen + '/candidate-kujo'}
hashes = binaries.transform_values { |p| Digest::SHA256.file(p).hexdigest }
raise 'baselines are not byte-identical' unless hashes['baseline_a'] == hashes['baseline_b']
config = File.expand_path('benchmarks/langarena/test.json')
port = frozen + '/timings/timed-port.kujo'
cases = JSON.parse(File.read(config)).to_h { |row| [row.fetch('name'), row] }
inputs = (binaries.values + [port, config, File.expand_path(__FILE__)]).to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
names = ARGV.drop(2)
names = %w[Sort::Quick Sort::Merge] if names.empty?
raise 'unknown or repeated workload' unless names.uniq == names && names.all? { |name| cases.key?(name) }
orders = binaries.keys.permutation.to_a
FileUtils.mkdir_p(out)
FileUtils.cp(port, out + '/timed-port.kujo')
report = {status: 'running', binaries: binaries, inputs: inputs, orders: orders, rows: [], scope: 'VM workload diagnostic: twelve counterbalanced triplets and one excluded warmup, with byte-identical baseline A/B. Not clean-host or accepted performance.'}
save = -> { File.write(out + '/report.json', JSON.pretty_generate(report)) }
begin
  (-1...12).each do |round|
    names.rotate([round, 0].max % names.size).each do |name|
      orders[[round, 0].max % orders.size].each do |variant|
        inputs.each { |p, h| raise 'input drift' unless Digest::SHA256.file(p).hexdigest == h }
        label = "#{round}-#{names.index(name)}-#{variant}"
        argv = [binaries.fetch(variant), 'run', '--scheduler-no-timeout', port, config, name]
        start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        pid = Process.spawn(*argv, out: out + '/' + label + '.out', err: out + '/' + label + '.err', pgroup: true)
        status = nil
        loop do
          if (done = Process.wait2(pid, Process::WNOHANG))
            status = done.last; break
          end
          if Process.clock_gettime(Process::CLOCK_MONOTONIC) - start > 300
            Process.kill('KILL', -pid); Process.wait(pid)
            raise "timeout #{label}"
          end
          sleep 0.001
        end
        wall = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start
        stdout = File.read(out + '/' + label + '.out'); stderr = File.read(out + '/' + label + '.err')
        timer = stdout.lines.grep(/^@comparison_ns /)
        raise 'invalid output' unless status.success? && stderr.empty? && timer.size == 1 && stdout.lines.map(&:strip) == [timer[0].strip, "#{name}: OK #{cases.fetch(name).fetch('checksum')}", 'Summary: 1, 1, 0']
        kernel = Integer(timer[0].split.last); raise 'invalid timer' unless kernel > 0
        report[:rows] << {round: round, name: name, variant: variant, argv: argv, exit: status.exitstatus, stdout: stdout, stderr: stderr, kernel_ns: kernel, wall_seconds: wall}
        save.call
      end
    end
  end
  report[:status] = 'collected-review-pending'
ensure
  save.call
end
