# Characterize collection replacement below dictionary, struct and array roots.
# Allocation traffic is deterministic evidence; this is not a timing benchmark.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'

abort 'Expected PROBE BINARY NEW_OUTPUT' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable' unless File.executable?(p) }
FileUtils.mkdir_p(output)
report = {
  status: 'running',
  scope: 'Nested collection-valued replacement with original and fresh snapshots; characterization only.',
  identities: [probe, binary, __FILE__].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] },
  sources: {}, rows: [], semantics: []
}
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[array dictionary].product(%w[dict struct array], [128, 2048], %w[once fresh], [16, 32]).each do |kind, shape, width, snapshot, calls|
    directory = output + "/#{kind}-#{shape}-#{width}-#{snapshot}-#{calls}"
    FileUtils.mkdir_p(directory)
    value = ->(n) { kind == 'array' ? "[#{n}]" : "{\"value\": #{n}}" }
    access = kind == 'array' ? '[0]' : '["value"]'
    root, row = case shape
                when 'dict' then ['mut state := {"row": seed}', 'state["row"]']
                when 'struct' then ['struct State { row: dict } mut state := State { row: seed }', 'state.row']
                else ['mut state := [seed]', 'state[0]']
                end
    owner = <<~KUJO
      mut seed := {}
      mut key := 0
      while key < #{width} { seed[key] = #{value.call('0')} key += 1 }
      #{root}
      seed = null
      export func snapshot() { return state }
      export func edit(amount) { #{row}[0] = #{value.call('amount')} return amount }
    KUJO
    saved, original, final = %w[saved original final].map { |name| row.sub('state', name) }
    source = <<~KUJO
      from owner import snapshot, edit
      let original := snapshot()
      mut i := 0
      while i < #{calls} {
        #{snapshot == 'fresh' ? 'let saved := snapshot()' : ''}
        assert_equal(edit(i + 1), i + 1)
        #{snapshot == 'fresh' ? "assert_equal(#{saved}[0]#{access}, i) assert_equal(len(#{saved}), #{width})" : ''}
        i += 1
      }
      let final := snapshot()
      assert_equal(len(#{original}), #{width})
      assert_equal(len(#{final}), #{width})
      mut key := 0
      while key < #{width} {
        assert_equal(len(#{original}[key]), 1)
        assert_equal(#{original}[key]#{access}, 0)
        assert_equal(len(#{final}[key]), 1)
        if key == 0 { assert_equal(#{final}[key]#{access}, #{calls}) }
        else { assert_equal(#{final}[key]#{access}, 0) }
        key += 1
      }
      print("NESTED COLLECTION REPLACEMENT PASS")
    KUJO
    File.write(directory + '/owner.kujo', owner)
    File.write(directory + '/main.kujo', source)
    sh, mh = [source, owner].map { |s| Digest::SHA256.hexdigest(s) }
    report[:sources][sh] = source
    report[:sources][mh] = owner
    meta = {kind: kind, shape: shape, width: width, snapshot: snapshot, calls: calls, source_sha256: sh, module_sha256: mh}
    %w[vm interpreter jit].each do |mode|
      args = [binary, 'run', directory + '/main.kujo', '--untrusted', '--scheduler-no-timeout']
      args << '--' + mode unless mode == 'vm'
      stdout, stderr, status = Open3.capture3(*args)
      raise stdout + stderr unless status.success? && stdout == "NESTED COLLECTION REPLACEMENT PASS\n"
      report[:semantics] << meta.merge(mode: mode, exit: status.exitstatus)
      save.call
    end
    %w[vm jit].each do |mode|
      stdout, stderr, status = Open3.capture3(probe, mode, directory + '/main.kujo')
      raise stdout + stderr unless status.success? && stdout == "NESTED COLLECTION REPLACEMENT PASS\n"
      counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
      raise stderr unless counts
      report[:rows] << meta.merge(mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
      save.call
    end
  end
  raise 'Coverage' unless report[:rows].size == 96 && report[:semantics].size == 144
  report[:slopes] = report[:rows].group_by { |x| x.values_at(:kind, :shape, :width, :snapshot, :mode) }.map do |group, rows|
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
