require 'json'
require 'digest'
require 'open3'
require 'fileutils'
require 'time'

abort 'usage: ruby compare-native-calls.rb BASELINE CANDIDATE OUTPUT [ROUNDS]' unless (3..4).cover?(ARGV.length)
baseline, candidate, directory = ARGV.take(3).map { |path| File.expand_path(path) }
rounds = Integer(ARGV.fetch(3, '7'))
raise 'rounds must be positive' unless rounds > 0
binaries = { 'baseline' => baseline, 'candidate' => candidate }
admission_notice = "JIT opt-in requested; unsupported bytecode regions will use the VM (unsupported opcode in chunk '<script>' at instruction 0: MakeClosure(0)).\n"
raise 'output exists' if File.exist?(directory)
source = 'benchmarks/langarena/probes/native-call-scaling.kujo'
files = [baseline, candidate, source, __FILE__].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
altered_execution = ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/)
raise "diagnostic/JIT environment overrides: #{altered_execution.join(', ')}" unless altered_execution.empty?
verify = lambda do
  files.each { |p,h| raise "identity drift: #{p}" unless Digest::SHA256.file(p).hexdigest == h }
  processes, status = Open3.capture2('ps', '-Ao', 'comm=')
  raise 'active build or missing process inventory' unless status.success? && processes.lines.none? { |l| %w[cargo rustc].include?(File.basename(l.strip)) }
end
verify.call
FileUtils.mkdir_p(directory)
state = {status: 'running', pid: Process.pid, started_at: Time.now.utc.iso8601,
         scope: 'Paired local call scaling diagnostic; not clean-host score or whole-suite attribution',
         files: files, calls: 32768, rounds: rounds, rows: []}
save = lambda do
  File.write(directory + '/state.json.tmp', JSON.pretty_generate(state) + "\n")
  File.rename(directory + '/state.json.tmp', directory + '/state.json')
end
save.call
begin
  cells = (0..4).map { |a| ['scalar',a,8] } + (1..4).to_a.product([8,256,8192]).map { |a,w| ['array',a,w] }
  cells.each do |kind,arity,width|
    rounds.times do |round|
      configurations = binaries.keys.product(%w[vm jit]).rotate(round % 4)
      configurations.reverse! if round.odd?
      configurations.each do |revision, mode|
        binary = binaries.fetch(revision)
        verify.call
        argv = [binary, 'run'] + (mode == 'jit' ? ['--jit'] : []) + [source,kind,arity.to_s,width.to_s,'32768','measure']
        index = state[:rows].size
        stdout, stderr = ["#{directory}/#{index}.out", "#{directory}/#{index}.err"]
        pid = Process.spawn(*argv, out: stdout, err: stderr, pgroup: true)
        state[:child_pid] = pid
        state[:argv] = argv
        save.call
        start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        timed_out = false
        result = nil
        loop do
          if (waited = Process.wait2(pid, Process::WNOHANG))
            result = waited.last
            break
          end
          if Process.clock_gettime(Process::CLOCK_MONOTONIC) - start > 120
            timed_out = true
            Process.kill('TERM', -pid) rescue nil
            sleep 0.2
            Process.kill('KILL', -pid) rescue nil
            result = Process.wait2(pid).last
            break
          end
          sleep 0.02
        end
        state.delete(:child_pid)
        lines = File.read(stdout).lines.map(&:strip)
        diagnostics = File.read(stderr)
        diagnostics_valid = diagnostics.empty? || (mode == 'jit' && diagnostics == admission_notice)
        valid = result.success? && !timed_out && diagnostics_valid && lines.size == 2 && lines[0].match?(/\A@call_ns [0-9]+\z/) && lines[1] == 'native-call-scaling: OK'
        ns = valid ? Integer(lines[0].split.last) : nil
        valid &&= ns > 0
        state[:rows] << {kind: kind, arity: arity, width: width, revision: revision, runtime: mode, round: round+1,
                         argv: argv, exit: result.exitstatus, signal: result.termsig,
                         timed_out: timed_out, passed: !!valid, kernel_ns: ns, stdout: stdout, stderr: stderr,
                         stdout_sha256: Digest::SHA256.file(stdout).hexdigest,
                         stderr_sha256: Digest::SHA256.file(stderr).hexdigest}
        save.call
        raise 'comparison control failed' unless valid
        verify.call
      end
    end
  end
  raise 'coverage' unless state[:rows].size == cells.size * rounds * 4
  state[:status] = 'measured-review-pending'
  state[:finished_at] = Time.now.utc.iso8601
  save.call
rescue => error
  state[:status] = 'failed'
  state[:error] = error.message
  save.call
  raise
end
