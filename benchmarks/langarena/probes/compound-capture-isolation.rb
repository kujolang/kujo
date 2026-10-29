# Isolate indexed arithmetic from captured scalar reads and writes.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected PROBE BINARY NEW_OUTPUT' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable' unless File.executable?(p) }
FileUtils.mkdir_p(output)
report = {status: 'running', scope: 'Eight indexed writes with independent scalar capture effects; allocation characterization only.', identities: [probe, binary, __FILE__].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }, sources: {}, rows: [], semantics: []}
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[direct compound].product(%w[parameter read_only write_only increment_return], [128, 2048], %w[once fresh], [16, 32]).each do |operation, counter, width, snapshot, calls|
    directory = output + "/#{operation}-#{counter}-#{width}-#{snapshot}-#{calls}"
    FileUtils.mkdir_p(directory)
    update = {'parameter' => '', 'read_only' => '', 'write_only' => 'counter = amount', 'increment_return' => 'counter += 1'}.fetch(counter)
    result = %w[read_only increment_return].include?(counter) ? 'counter' : 'amount'
    expected_return = counter == 'read_only' ? '0' : 'i + 1'
    expected_counter = %w[write_only increment_return].include?(counter) ? calls : 0
    rhs = operation == 'compound' ? 'state[j] + 1' : 'amount'
    owner = <<~KUJO
      mut state := {}
      mut key := 0
      while key < #{width} { state[key] = 0 key += 1 }
      mut counter := 0
      export func snapshot() { return state }
      export func counter_value() { return counter }
      export func edit(amount) {
        #{update}
        mut j := 0
        while j < 8 { state[j] = #{rhs} j += 1 }
        return #{result}
      }
    KUJO
    source = <<~KUJO
      from owner import snapshot, counter_value, edit
      let original := snapshot()
      mut i := 0
      while i < #{calls} {
        #{snapshot == 'fresh' ? 'let saved := snapshot()' : ''}
        assert_equal(edit(i + 1), #{expected_return})
        #{snapshot == 'fresh' ? "assert_equal(len(saved), #{width}) mut j := 0 while j < 8 { assert_equal(saved[j], i) j += 1 }" : ''}
        i += 1
      }
      let final := snapshot()
      assert_equal(len(original), #{width})
      assert_equal(len(final), #{width})
      assert_equal(counter_value(), #{expected_counter})
      mut key := 0
      while key < #{width} {
        assert_equal(original[key], 0)
        if key < 8 { assert_equal(final[key], #{calls}) }
        else { assert_equal(final[key], 0) }
        key += 1
      }
      print("COMPOUND CAPTURE PASS")
    KUJO
    File.write(directory + '/owner.kujo', owner)
    File.write(directory + '/main.kujo', source)
    sh, mh = [source, owner].map { |s| Digest::SHA256.hexdigest(s) }
    report[:sources][sh] = source
    report[:sources][mh] = owner
    meta = {operation: operation, counter: counter, width: width, snapshot: snapshot, calls: calls, source_sha256: sh, module_sha256: mh}
    %w[vm interpreter jit].each do |mode|
      args = [binary, 'run', directory + '/main.kujo', '--untrusted', '--scheduler-no-timeout']
      args << '--' + mode unless mode == 'vm'
      stdout, stderr, status = Open3.capture3(*args)
      raise stdout + stderr unless status.success? && stdout == "COMPOUND CAPTURE PASS\n"
      report[:semantics] << meta.merge(mode: mode, exit: status.exitstatus)
      save.call
    end
    %w[vm jit].each do |mode|
      stdout, stderr, status = Open3.capture3(probe, mode, directory + '/main.kujo')
      raise stdout + stderr unless status.success? && stdout == "COMPOUND CAPTURE PASS\n"
      counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
      raise stderr unless counts
      report[:rows] << meta.merge(mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
      save.call
    end
  end
  raise 'Coverage' unless report[:rows].size == 128 && report[:semantics].size == 192
  report[:slopes] = report[:rows].group_by { |x| x.values_at(:operation, :counter, :width, :snapshot, :mode) }.map do |group, rows|
    a, b = rows.sort_by { |x| x[:calls] }
    {group: group, calls: [a[:calls], b[:calls]], requests: (b[:requests] - a[:requests]) / 16.0, bytes: (b[:bytes] - a[:bytes]) / 16.0}
  end
  report[:identities].each { |p, h| raise 'Artifact drift' unless Digest::SHA256.file(p).hexdigest == h }
  report[:status] = 'characterized'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
