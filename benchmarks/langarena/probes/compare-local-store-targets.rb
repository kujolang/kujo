# Unchanged kernels, timing-only runner copy, sequential alternating pairs.
require 'json'
require 'digest'
require 'fileutils'
raise 'usage: BASELINE CANDIDATE NEW_OUTPUT' unless ARGV.size == 3
binaries = %w[baseline candidate].zip(ARGV.first(2).map { |p| File.expand_path(p) }).to_h
out = File.expand_path(ARGV[2]); raise 'output exists' if File.exist?(out)
raise 'diagnostic environment' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
source = File.read('benchmarks/langarena/port.kujo')
first = '            while i < cfg["iterations"] {'
last = '            let actual := u32(checksum(state))'
raise 'timer anchors' unless source.scan(first).size == 1 && source.scan(last).size == 1
source = source.sub(first, "            let comparison_started := time_ns()\n" + first)
source = source.sub(last, "            print(\"@comparison_ns \" + str(time_ns() - comparison_started))\n" + last)
FileUtils.mkdir_p(out)
port = out + '/timed-port.kujo'; File.write(port, source)
config = File.expand_path('benchmarks/langarena/test.json')
cases = JSON.parse(File.read(config)).to_h { |r| [r.fetch('name'), r] }
names = %w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::TextRaytracer Matmul::T4 Matmul::T8 Matmul::T16 Base64::Encode Base64::Decode Etc::CacheSimulation Maze::Generator Maze::BFS Maze::AStar Compress::BWTEncode Compress::HuffEncode Compress::HuffDecode]
cells = names.product(%w[vm jit])
files = (binaries.values + [port, config, File.expand_path(__FILE__)]).to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
report = {status: 'running', binaries: binaries, files: files, cells: cells, rows: [], scope: 'Test-size kernels; seven alternating paired rounds, one excluded process warmup. No clean-host or competitor score.'}
save = -> { File.write(out + '/report.json', JSON.pretty_generate(report)) }
begin
  (-1...7).each do |round|
    cells.rotate([round, 0].max).each do |name, runtime|
      index = cells.index([name, runtime])
      order = (round + index).even? ? %w[baseline candidate] : %w[candidate baseline]
      order.each do |variant|
        files.each { |p, h| raise 'input drift' unless Digest::SHA256.file(p).hexdigest == h }
        label = [round, index, variant].join('-')
        argv = [binaries.fetch(variant), 'run', '--scheduler-no-timeout']
        argv << '--jit' if runtime == 'jit'
        argv += [port, config, name]
        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        pid = Process.spawn(*argv, out: out + '/' + label + '.out', err: out + '/' + label + '.err', pgroup: true)
        status = nil
        loop do
          if (done = Process.wait2(pid, Process::WNOHANG))
            status = done.last; break
          end
          if Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 300
            Process.kill('KILL', -pid); Process.wait(pid)
            raise "timeout #{label}"
          end
          sleep 0.001
        end
        wall = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
        stdout = File.read(out + '/' + label + '.out'); stderr = File.read(out + '/' + label + '.err')
        timer = stdout.lines.grep(/^@comparison_ns /)
        raise "invalid result #{label}" unless status.success? && timer.size == 1 && stdout.lines.map(&:strip) == [timer[0].strip, "#{name}: OK #{cases.fetch(name).fetch('checksum')}", 'Summary: 1, 1, 0']
        raise 'unexpected stderr' unless stderr.empty? || (runtime == 'jit' && stderr.match?(/\AJIT opt-in requested; unsupported bytecode regions will use the VM \([^\n]+\)\.\n\z/))
        kernel = Integer(timer[0].split.last); raise 'invalid timer' unless kernel > 0
        report[:rows] << {round: round, name: name, runtime: runtime, variant: variant, argv: argv, exit: status.exitstatus, stdout: stdout, stderr: stderr, kernel_ns: kernel, wall_seconds: wall}
        save.call
      end
    end
  end
  report[:status] = 'collected-review-pending'
ensure
  save.call
end
