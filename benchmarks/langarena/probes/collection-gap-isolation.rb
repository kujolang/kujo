# Characterize remaining collection-width costs; no timing or optimization acceptance.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected ACCEPTED_PROBE ACCEPTED_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
FileUtils.mkdir_p(output)
shapes = %w[dict_one_int dict_two_int dict_one_float struct_row_one struct_row_two dict_row_two array_row_one deep_row_one dict_dict_one struct_dict_one]
r = { status: 'running', scope: 'Added-call allocation characterization on accepted runtime; no candidate acceptance',
      runner_sha256: Digest::SHA256.file(__FILE__).hexdigest, probe_sha256: Digest::SHA256.file(probe).hexdigest,
      binary_sha256: Digest::SHA256.file(binary).hexdigest, rows: [], semantics: [], sources: {} }
save = -> { File.write(output + '/report.json', JSON.pretty_generate(r) + "\n") }
save.call
begin
  shapes.product([2, 128, 2048], %w[once fresh]).each do |shape, width, snapshot|
    dir = "#{output}/#{shape}-#{width}-#{snapshot}"
    FileUtils.mkdir_p(dir)
    float = shape == 'dict_one_float'
    dictionary = "mut seed := {\"0\": #{float ? '0.0' : '0'}}\nmut k := 1\nwhile k < #{width} { seed[k] = k k += 1 }\n"
    root, first, second = case shape
      when 'dict_one_int', 'dict_two_int', 'dict_one_float'
        [dictionary + 'mut state := seed', 'state[0]', 'state[1]']
      when 'struct_row_one', 'struct_row_two'
        ["struct State { row: array }\nmut state := State { row: range(0, #{width}) }", 'state.row[0]', 'state.row[1]']
      when 'dict_row_two'
        ["mut state := {\"row\": range(0, #{width})}", 'state["row"][0]', 'state["row"][1]']
      when 'array_row_one'
        ["mut state := [range(0, #{width})]", 'state[0][0]', 'state[0][1]']
      when 'deep_row_one'
        ["mut state := {\"inner\": {\"row\": range(0, #{width})}}", 'state["inner"]["row"][0]', 'state["inner"]["row"][1]']
      when 'dict_dict_one'
        [dictionary + 'mut state := {"row": seed}', 'state["row"][0]', 'state["row"][1]']
      when 'struct_dict_one'
        [dictionary + "struct State { row: dict }\nmut state := State { row: seed }", 'state.row[0]', 'state.row[1]']
    end
    # Do not retain the construction alias across calls: the explicit snapshots
    # below are the intended ownership controls, not an accidental seed alias.
    root += "\nseed = null" if shape.start_with?('dict_one', 'dict_two') || %w[dict_dict_one struct_dict_one].include?(shape)
    two = %w[dict_two_int struct_row_two dict_row_two].include?(shape)
    increment = float ? '1.0' : '1'
    mod = "#{root}\nexport func snapshot() { return state }\nexport func edit() { #{first} += #{increment}\n#{two ? second + ' += 1' : ''}\nreturn #{first} }\n"
    File.write(dir + '/owner.kujo', mod)
    module_hash = Digest::SHA256.hexdigest(mod)
    r[:sources][module_hash] = mod
    [16, 32].each do |calls|
      expression = ->(name, value) { value.sub('state', name) }
      initial = float ? '0.0' : '0'
      saved = snapshot == 'fresh' ? "let saved := snapshot()\n" : ''
      saved_check = snapshot == 'fresh' ? "assert_equal(#{expression.call('saved', first)}, i * #{increment})\nassert_equal(#{expression.call('saved', second)}, #{two ? 'i + 1' : '1'})" : ''
      source = "from owner import edit, snapshot\nlet original := snapshot()\nmut i := 0\nwhile i < #{calls} { #{saved}assert_equal(edit(), (i + 1) * #{increment})\n#{saved_check}\ni += 1 }\nlet final := snapshot()\nassert_equal(#{expression.call('original', first)}, #{initial})\nassert_equal(#{expression.call('original', second)}, 1)\nassert_equal(#{expression.call('final', first)}, #{calls} * #{increment})\nassert_equal(#{expression.call('final', second)}, #{two ? calls + 1 : 1})\nprint(\"COLLECTION GAP PASS\")\n"
      path = dir + "/calls-#{calls}.kujo"
      File.write(path, source)
      source_hash = Digest::SHA256.hexdigest(source)
      r[:sources][source_hash] = source
      metadata = { shape: shape, width: width, snapshot: snapshot, calls: calls,
                   source_sha256: source_hash, module_sha256: module_hash }
      %w[vm jit].each do |mode|
        stdout, stderr, status = Open3.capture3(probe, mode, path)
        raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "COLLECTION GAP PASS\n"
        counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
        raise stderr unless counts
        r[:rows] << metadata.merge(mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
        save.call
      end
      %w[vm interpreter jit].each do |mode|
        args = [binary, 'run', path, '--untrusted']
        args << '--' + mode unless mode == 'vm'
        stdout, stderr, status = Open3.capture3(*args)
        raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "COLLECTION GAP PASS\n"
        r[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
        save.call
      end
    end
  end
  raise 'Coverage' unless r[:rows].size == 240 && r[:semantics].size == 360
  r[:deltas] = r[:rows].group_by { |x| x.values_at(:shape, :width, :snapshot, :mode) }.map do |key, rows|
    first, last = rows.sort_by { |x| x[:calls] }
    { shape: key[0], width: key[1], snapshot: key[2], mode: key[3],
      requests_per_added_call: (last[:requests] - first[:requests]) / 16.0,
      bytes_per_added_call: (last[:bytes] - first[:bytes]) / 16.0 }
  end
  r[:status] = 'characterized'
  save.call
rescue => error
  r[:status] = 'failed'; r[:error] = error.message; save.call; raise
end
