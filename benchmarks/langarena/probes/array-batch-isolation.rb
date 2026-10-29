# Isolate general collection batches from owner width and required snapshots.
# Repository-maintenance probe; generated benchmark programs execute in Kujo.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected PROBE BINARY NEW_OUTPUT' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable' unless File.executable?(p) }
FileUtils.mkdir_p(output)
report = {status: 'running', pid: Process.pid, scope: 'Counted 3/8/64-entry array, mixed array/dictionary and struct batches; allocation characterization only.', identities: [probe, binary, __FILE__].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }, sources: {}, rows: [], semantics: []}
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[array mixed struct].product([3, 8, 64], [64, 2048], %w[once fresh], [1, 2, 16, 32]).each do |payload, writes, width, snapshot, calls|
    directory = output + "/#{payload}-#{writes}-#{width}-#{snapshot}-#{calls}"
    FileUtils.mkdir_p(directory)
    seed = payload == 'struct' ? 'Item { value: 0 }' : '[0]'
    update = case payload
             when 'array'; 'state[j] = [amount]'
             when 'struct'; 'state[j] = Item { value: amount }'
             else; 'if j % 2 == 0 { state[j] = [amount] } else { state[j] = {"value": amount} }'
             end
    check = lambda do |name, expected|
      case payload
      when 'array'; "assert_equal(#{name}[j][0], #{expected})"
      when 'struct'; "assert_equal(#{name}[j].value, #{expected})"
      else; "if j % 2 == 0 { assert_equal(#{name}[j][0], #{expected}) } else { assert_equal(#{name}[j][\"value\"], #{expected}) }"
      end
    end
    zero = ->(name) { "assert_equal(#{name}[j]#{payload == 'struct' ? '.value' : '[0]'}, 0)" }
    owner = <<~KUJO
      struct Item { value: int }
      mut state := []
      mut key := 0
      while key < #{width} { state = push(state, #{seed}) key += 1 }
      export func snapshot() { return state }
      export func edit(amount, limit) {
        mut j := 0
        while j < limit { #{update} j += 1 }
        return amount
      }
    KUJO
    source = <<~KUJO
      from owner import snapshot, edit
      let original := snapshot()
      mut i := 0
      while i < #{calls} {
        #{snapshot == 'fresh' ? 'let saved := snapshot()' : ''}
        assert_equal(edit(i + 1, #{writes}), i + 1)
        #{snapshot == 'fresh' ? "assert_equal(len(saved), #{width}) mut j := 0 while j < #{writes} { if i == 0 { #{zero.call('saved')} } else { #{check.call('saved', 'i')} } j += 1 }" : ''}
        i += 1
      }
      let final := snapshot()
      assert_equal(len(original), #{width})
      assert_equal(len(final), #{width})
      mut j := 0
      while j < #{width} {
        #{zero.call('original')}
        if j < #{writes} { #{check.call('final', calls)} }
        else { #{zero.call('final')} }
        j += 1
      }
      print("ARRAY BATCH PASS")
    KUJO
    File.write(directory + '/owner.kujo', owner)
    File.write(directory + '/main.kujo', source)
    sh, mh = [source, owner].map { |s| Digest::SHA256.hexdigest(s) }
    report[:sources][sh] = source
    report[:sources][mh] = owner
    meta = {payload: payload, writes: writes, width: width, snapshot: snapshot, calls: calls, source_sha256: sh, module_sha256: mh}
    %w[vm interpreter jit].each do |mode|
      args = [binary, 'run', directory + '/main.kujo', '--untrusted', '--scheduler-no-timeout']
      args << '--' + mode unless mode == 'vm'
      stdout, stderr, status = Open3.capture3(*args)
      raise stdout + stderr unless status.success? && stdout == "ARRAY BATCH PASS\n"
      report[:semantics] << meta.merge(mode: mode, argv: args, exit: status.exitstatus)
    end
    %w[vm jit].each do |mode|
      args = [probe, mode, directory + '/main.kujo']
      stdout, stderr, status = Open3.capture3(*args)
      raise stdout + stderr unless status.success? && stdout == "ARRAY BATCH PASS\n"
      counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
      raise stderr unless counts
      report[:rows] << meta.merge(mode: mode, argv: args, exit: status.exitstatus, requests: counts[1].to_i, bytes: counts[2].to_i)
    end
    save.call
  end
  raise 'Coverage' unless report[:rows].size == 288 && report[:semantics].size == 432
  report[:identities].each { |p, h| raise 'Artifact drift' unless Digest::SHA256.file(p).hexdigest == h }
  report[:status] = 'characterized'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
