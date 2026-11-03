# Paired process-wall diagnostic. No runtime instrumentation or speed acceptance.
require 'json'
require 'digest'
require 'fileutils'
require 'time'
abort 'usage: BASELINE_BINARY CANDIDATE_BINARY NEW_OUTPUT' unless ARGV.size == 3
binaries = %w[baseline candidate].zip(ARGV.first(2).map { |p| File.expand_path(p) }).to_h
directory = File.expand_path(ARGV[2])
raise 'output exists' if File.exist?(directory)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
source = File.expand_path('benchmarks/langarena/probes/jit-async-worker-economics.kujo')
files = (binaries.values + [source, File.expand_path(__FILE__)]).uniq.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
verify = -> { files.each { |p, hash| raise "input drift #{p}" unless Digest::SHA256.file(p).hexdigest == hash } }
FileUtils.mkdir_p(directory)
report = {status: 'running', started_at: Time.now.utc.iso8601, files: files,
          binaries: binaries, rows: [],
          scope: 'Seven alternating paired rounds after one excluded warmup per cell/variant. Fixed 64 jobs per sample; process wall includes startup, compilation and checks. Uninstrumented task microbenchmark, not all50, production, clean-host or competitor performance.'}
save = -> { File.write(directory + '/report.json.tmp', JSON.pretty_generate(report)); File.rename(directory + '/report.json.tmp', directory + '/report.json') }
cells = %w[vm interpreter jit].flat_map do |runtime|
  [['direct', 1], ['parallel', 4], ['parallel', 8], ['parallel', 16]].map do |mode, workers|
    {runtime: runtime, mode: mode, workers: workers}
  end
end
begin
  (-1...7).each do |round|
    cells.rotate([round, 0].max % cells.size).each do |cell|
      order = (round + cells.index(cell)).even? ? %w[baseline candidate] : %w[candidate baseline]
      order.each do |variant|
        verify.call
        label = [round, cell[:runtime], cell[:mode], cell[:workers], variant].join('-')
        argv = [binaries.fetch(variant), 'run', '--scheduler-timeout-ms', '60000']
        argv << '--interpreter' if cell[:runtime] == 'interpreter'
        argv << '--jit' if cell[:runtime] == 'jit'
        argv += [source, cell[:mode], cell[:workers].to_s, '4']
        out = directory + '/' + label + '.out'
        err = directory + '/' + label + '.err'
        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        pid = Process.spawn(*argv, out: out, err: err, pgroup: true)
        status = nil
        timed_out = false
        loop do
          if (result = Process.wait2(pid, Process::WNOHANG))
            status = result.last
            break
          end
          if Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 120
            timed_out = true
            Process.kill('TERM', -pid) rescue nil
            sleep 0.2
            Process.kill('KILL', -pid) rescue nil
            status = Process.wait2(pid).last
            break
          end
          sleep 0.001
        end
        elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
        stdout = File.read(out)
        stderr = File.read(err)
        normal_jit_notice = cell[:runtime] == 'jit' && stderr.match?(/\AJIT opt-in requested; unsupported bytecode regions will use the VM \([^\n]+\)\.\n\z/)
        passed = status.success? && !timed_out && stdout.lines.map(&:strip) == ['async-worker-economics: OK'] && (stderr.empty? || normal_jit_notice)
        report[:rows] << cell.merge(round: round, warmup: round == -1, variant: variant,
          argv: argv, process_wall_seconds: elapsed, exit: status.exitstatus,
          signal: status.termsig, timed_out: timed_out, passed: passed,
          stdout: stdout, stderr: stderr)
        save.call
        raise "failed #{label}" unless passed
        verify.call
      end
    end
  end
  raise 'coverage' unless report[:rows].size == 192
  report[:comparisons] = cells.map do |cell|
    ratios = (0...7).map do |round|
      rows = report[:rows].select { |r| r[:round] == round && cell.all? { |key, value| r[key] == value } }
      times = rows.to_h { |r| [r[:variant], r[:process_wall_seconds]] }
      times.fetch('baseline') / times.fetch('candidate')
    end
    cell.merge(paired_speedups: ratios, median_speedup: ratios.sort[3], range: ratios.minmax)
  end
  report[:status] = 'collected-review-pending'
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  raise
ensure
  report[:finished_at] = Time.now.utc.iso8601
  save.call
end
puts JSON.pretty_generate(report[:comparisons])
