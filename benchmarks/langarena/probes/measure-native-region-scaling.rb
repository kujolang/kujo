# Diagnostic pairs only; ordinary CLI binaries, no instrumented timing.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
require 'time'

baseline, candidate, directory = ARGV
abort 'usage: ruby measure-native-region-scaling.rb BASELINE CANDIDATE OUTPUT' unless ARGV.size == 3
raise 'output already exists' if File.exist?(directory)
source = 'benchmarks/langarena/probes/native-region-scaling.kujo'
files = [baseline, candidate, source, __FILE__].uniq.to_h { |path| [path, Digest::SHA256.file(path).hexdigest] }
verify = lambda do
  files.each { |path, hash| raise "identity drift: #{path}" unless Digest::SHA256.file(path).hexdigest == hash }
  processes, status = Open3.capture2('ps', '-Ao', 'comm=')
  raise 'build activity or unavailable process inventory' unless status.success? && processes.lines.none? { |line| %w[cargo rustc].include?(File.basename(line.strip)) }
end
verify.call
FileUtils.mkdir_p(directory)
receipt = {status: 'running', pid: Process.pid, started_at: Time.now.utc.iso8601,
           scope: 'Local diagnostic kernel timing; no clean-host or competitor claim',
           files: files, rounds: 7, groups: 32768, rows: []}
save = lambda do
  File.write(directory + '/results.json.tmp', JSON.pretty_generate(receipt) + "\n")
  File.rename(directory + '/results.json.tmp', directory + '/results.json')
end
save.call
begin
  %w[integer-array negative-array float-array bytes integer-dictionary string-dictionary].each do |name|
    [8, 256, 8192].each do |width|
      %w[vm jit].each do |mode|
        7.times do |round|
          pair = [['baseline', baseline], ['candidate', candidate]]
          pair.reverse! if round.odd?
          pair.each do |label, binary|
            verify.call
            argv = [binary, 'run']
            argv << '--jit' if mode == 'jit'
            argv += [source, name, width.to_s, receipt[:groups].to_s, 'measure']
            index = receipt[:rows].size
            out_path, err_path = ["#{directory}/#{index}.out", "#{directory}/#{index}.err"]
            started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
            pid = Process.spawn(*argv, out: out_path, err: err_path, pgroup: true)
            outcome = nil
            timed_out = false
            loop do
              if (waited = Process.wait2(pid, Process::WNOHANG))
                outcome = waited.last
                break
              end
              if Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 60
                timed_out = true
                Process.kill('TERM', -pid) rescue nil
                sleep 0.2
                Process.kill('KILL', -pid) rescue nil
                outcome = Process.wait2(pid).last
                break
              end
              sleep 0.02
            end
            output = File.read(out_path)
            timers = output.lines.grep(/\A@region_ns [0-9]+\s*\z/)
            valid = outcome.success? && !timed_out && timers.size == 1 && output.lines.count { |line| line.strip == 'native-region-scaling: OK' } == 1
            ns = timers.size == 1 ? Integer(timers.first.split.last) : nil
            valid &&= ns && ns > 0
            receipt[:rows] << {case: name, width: width, runtime: mode, round: round + 1,
                              variant: label, argv: argv, exit: outcome.exitstatus,
                              signal: outcome.termsig, timed_out: timed_out, passed: !!valid,
                              kernel_ns: ns, stdout: out_path, stderr: err_path}
            save.call
            raise "failed control #{name}/#{width}/#{mode}/#{label}" unless valid
            verify.call
          end
        end
      end
    end
  end
  raise 'incomplete coverage' unless receipt[:rows].size == 504
  receipt[:status] = 'measured-review-pending'
  receipt[:finished_at] = Time.now.utc.iso8601
  save.call
rescue => error
  receipt[:status] = 'failed'
  receipt[:error] = error.message
  save.call
  raise
end
