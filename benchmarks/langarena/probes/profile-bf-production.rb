# Bounded, symbolized CPU attribution; deliberately not a benchmark score.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: KUJO_OR_--generate NEW_OUTPUT [SYMBOLIZER]' unless (2..3).cover?(ARGV.size)
binary, out, symbolizer = ARGV
raise 'symbolizer required' unless binary == '--generate' || (symbolizer && File.executable?(symbolizer))
out = File.expand_path(out)
raise 'output exists' if File.exist?(out)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
config_path = 'benchmarks/langarena/run.json'
common_path = 'benchmarks/langarena/src/00_common.kujo'
kernel_path = 'benchmarks/langarena/src/20_text_hash.kujo'
config = JSON.parse(File.read(config_path)).find { |row| row['name'] == 'Brainfuck::Array' }
raise 'production shape' unless config['warmup_iterations'] == 1000 && config['iterations'] == 1
# Independent byte interpreter for the small warmup checksum only.
code = config.fetch('warmup_program').bytes.select { |c| '+-<>[].,'.bytes.include?(c) }
stack = []; jumps = {}
code.each_with_index do |c,i|
  stack << i if c == 91
  if c == 93
    start = stack.pop; raise 'unmatched close' unless start
    jumps[start] = i; jumps[i] = start
  end
end
raise 'unmatched open' unless stack.empty?
tape = Hash.new(0); position = 0; pc = 0; checksum = 0; steps = 0
while pc < code.size
  steps += 1; raise 'warmup oracle bound' if steps > 1_000_000
  case code[pc]
  when 43 then tape[position] = (tape[position] + 1) & 255
  when 45 then tape[position] = (tape[position] - 1) & 255
  when 62 then position += 1
  when 60 then position = [position - 1, 0].max
  when 91 then pc = jumps.fetch(pc) if tape[position] == 0
  when 93 then pc = jumps.fetch(pc) if tape[position] != 0
  when 46 then checksum = (checksum * 4 + tape[position]) & 0xffffffff
  when 44 then raise 'unsupported input'
  end
  pc += 1
end
kernel = File.read(kernel_path).split('func bf_parse(code, start)', 2)
raise 'kernel extraction' unless kernel.size == 2
FileUtils.mkdir_p(out)
source = File.read(common_path) + "\n" + kernel.first + <<~KUJO
  mut profile_iteration := 0
  while profile_iteration < #{config.fetch('warmup_iterations')} {
      assert_equal(bf_array(#{JSON.generate(config.fetch('warmup_program'))}), #{checksum})
      profile_iteration += 1
  }
  print("profile-ready: production")
  let production_result := bf_array(#{JSON.generate(config.fetch('program'))})
  assert_equal(production_result, #{config.fetch('checksum')})
  print("profile-complete: checksum-ok")
KUJO
fixture = out + '/production.kujo'; File.write(fixture, source)
inputs = [config_path, common_path, kernel_path, __FILE__, fixture].map { |p| File.expand_path(p) }
inputs += [File.expand_path(binary), File.expand_path(symbolizer)] unless binary == '--generate'
report = {status: 'generated', files: inputs.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] },
          warmup_checksum: checksum, warmup_calls: 1000, rows: [],
          scope: 'Ten-second post-warmup samples of unchanged full production program; partial executions are terminated. Not elapsed performance or completed production evidence.'}
save = -> { File.write(out + '/report.json', JSON.pretty_generate(report)) }
save.call
exit if binary == '--generate'
binary = File.expand_path(binary)
begin
  %w[vm interpreter jit].each do |mode|
    stdout = out + '/' + mode + '.out'; stderr = out + '/' + mode + '.err'
    sample = out + '/' + mode + '.sample.txt'
    argv = [binary, 'run']; argv << '--interpreter' if mode == 'interpreter'; argv << '--jit' if mode == 'jit'; argv << fixture
    pid = Process.spawn(*argv, out: stdout, err: stderr)
    child_status = nil
    begin
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 300
      loop do
        if (ended = Process.wait2(pid, Process::WNOHANG))
          child_status = ended.last
          raise 'child completed before sample attachment'
        end
        break if File.read(stdout).lines.include?("profile-ready: production\n")
        raise 'warmup deadline exceeded' if Process.clock_gettime(Process::CLOCK_MONOTONIC) > deadline
        sleep 0.05
      end
      output, error, status = Open3.capture3('/usr/bin/sample', pid.to_s, '10', '5', '-file', sample)
      File.write(sample + '.collector', output + error)
      raise 'sampler failure' unless status.success? && File.file?(sample)
      raw = File.read(sample)
      text, symbol_error, symbol_status = Open3.capture3(File.expand_path(symbolizer), stdin_data: raw)
      raise 'symbol resolution failed' unless symbol_status.success? && symbol_error.empty?
      File.write(sample + '.resolved', text)
      raise 'sample identity' unless text.include?("[#{pid}]") && text.include?('Call graph:')
      # Require a named Kujo frame, not just libc symbols in a stripped executable.
      raise 'Kujo symbols missing' unless text.match?(/kujo::(?:vm|interpreter|jit)::/)
      worker_samples = text.lines.filter_map { |line| line[/^    (\d+) Thread_.*: kujo-(?:vm|interpreter)-runner/, 1]&.to_i }.sum
      raise "insufficient worker samples: #{worker_samples}" if worker_samples < 50
      report[:rows] << {worker_samples: worker_samples, runtime: mode, pid: pid, argv: argv, sample: sample, sampler_exit: status.exitstatus}
    ensure
      unless child_status
        if (ended = Process.wait2(pid, Process::WNOHANG))
          child_status = ended.last
        else
          Process.kill('TERM', pid)
          child_status = Process.wait2(pid).last
        end
      end
      report[:rows].last[:child_exit] = child_status.exitstatus if report[:rows].last && report[:rows].last[:pid] == pid
      report[:rows].last[:child_signal] = child_status.termsig if report[:rows].last && report[:rows].last[:pid] == pid
      save.call
    end
  end
  report[:files].each { |path, hash| raise 'input drift' unless Digest::SHA256.file(path).hexdigest == hash }
  report[:status] = 'collected-review-pending'
rescue => error
  report[:status] = 'failed'; report[:error] = error.message
  raise
ensure
  save.call
end
