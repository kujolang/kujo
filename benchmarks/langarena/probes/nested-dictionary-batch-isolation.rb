# Isolate bounded and overflowing nested dictionary batches, including insertion.
# Allocation characterization only; snapshots remain semantic controls.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected ACCEPTED_PROBE ACCEPTED_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 3
probe, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[probe, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
FileUtils.mkdir_p(output)
shapes = %w[dict_numeric dict_string struct_numeric struct_string array_numeric array_string].product(%w[one two four five insert]).map { |a,b| a + "_" + b }
r = { status: 'running', scope: 'Added-call allocation characterization on accepted runtime; no candidate acceptance',
      runner_sha256: Digest::SHA256.file(__FILE__).hexdigest, probe_sha256: Digest::SHA256.file(probe).hexdigest,
      binary_sha256: Digest::SHA256.file(binary).hexdigest, rows: [], semantics: [], sources: {} }
save = -> { File.write(output + '/report.json', JSON.pretty_generate(r) + "\n") }
save.call
begin
  shapes.product([128, 2048], %w[once fresh]).each do |shape, width, snapshot|
    dir = "#{output}/#{shape}-#{width}-#{snapshot}"
    FileUtils.mkdir_p(dir)
    owner, key_kind, pattern = shape.split('_')
    numeric = key_kind == 'numeric'
    seed_key = numeric ? 'k' : '"item" + str(k)'
    dictionary = "mut seed := {}\nmut k := 0\nwhile k < #{width} { seed[#{seed_key}] = k k += 1 }\n"
    root, row = case owner
      when 'dict' then [dictionary + 'mut state := {"row": seed}', 'state["row"]']
      when 'struct' then [dictionary + "struct State { row: dict }\nmut state := State { row: seed }", 'state.row']
      when 'array' then [dictionary + 'mut state := [seed]', 'state[0]']
    end
    root += "\nseed = null"
    first = row + (numeric ? '[0]' : '["item0"]')
    second = row + (numeric ? '[1]' : '["item1"]')
    float = false
    two = %w[two four five].include?(pattern)
    increment = '1'
    count = {'one'=>1,'two'=>2,'four'=>4,'five'=>5,'insert'=>1}.fetch(pattern)
    access = ->(n) { row + (numeric ? "[#{n}]" : "[\"item#{n}\"]") }
    updates = (0...count).map { |n| "#{access.call(n)} += 1" }.join("\n")
    updates += "\n#{row}[\"added\"] = #{first}" if pattern == 'insert'
    # A full inline journal must still coalesce a repeated write to its first key.
    updates += "\n#{first} = #{first}"
    mod = "#{root}\nexport func snapshot() { return state }\nexport func edit() { #{updates}\nreturn #{first} }\n"
    File.write(dir + '/owner.kujo', mod)
    module_hash = Digest::SHA256.hexdigest(mod)
    r[:sources][module_hash] = mod
    [16, 32].each do |calls|
      expression = ->(name, value) { value.sub('state', name) }
      initial = float ? '0.0' : '0'
      saved = snapshot == 'fresh' ? "let saved := snapshot()\n" : ''
      saved_check = snapshot == 'fresh' ? "assert_equal(#{expression.call('saved', first)}, i * #{increment})\nassert_equal(#{expression.call('saved', second)}, #{two ? 'i + 1' : '1'})" : ''
      source = "from owner import edit, snapshot\nlet original := snapshot()\nmut i := 0\nwhile i < #{calls} { #{saved}assert_equal(edit(), (i + 1) * #{increment})\n#{saved_check}\ni += 1 }\nlet final := snapshot()\nassert_equal(#{expression.call('original', first)}, #{initial})\nassert_equal(#{expression.call('original', second)}, 1)\nassert_equal(#{expression.call('final', first)}, #{calls} * #{increment})\nassert_equal(#{expression.call('final', second)}, #{two ? calls + 1 : 1})\nprint(\"NESTED DICTIONARY PASS\")\n"
      assertions = (2...count).map do |n|
        "assert_equal(#{expression.call('original', access.call(n))}, #{n})\nassert_equal(#{expression.call('final', access.call(n))}, #{calls+n})"
      end.join("\n")
      if pattern == 'insert'
        assertions += "\nassert_equal(has_key(#{expression.call('original', row)}, \"added\"), 0)\nassert_equal(#{expression.call('final', row)}[\"added\"], #{calls})"
      end
      source = source.sub('print("NESTED DICTIONARY PASS")', assertions + "\nprint(\"NESTED DICTIONARY PASS\")")
      path = dir + "/calls-#{calls}.kujo"
      File.write(path, source)
      source_hash = Digest::SHA256.hexdigest(source)
      r[:sources][source_hash] = source
      metadata = { shape: shape, width: width, snapshot: snapshot, calls: calls,
                   source_sha256: source_hash, module_sha256: module_hash }
      %w[vm jit].each do |mode|
        stdout, stderr, status = Open3.capture3(probe, mode, path)
        raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "NESTED DICTIONARY PASS\n"
        counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
        raise stderr unless counts
        r[:rows] << metadata.merge(mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
        save.call
      end
      %w[vm interpreter jit].each do |mode|
        args = [binary, 'run', path, '--untrusted']
        args << '--' + mode unless mode == 'vm'
        stdout, stderr, status = Open3.capture3(*args)
        raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "NESTED DICTIONARY PASS\n"
        r[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
        save.call
      end
    end
  end
  raise 'Coverage' unless r[:rows].size == 480 && r[:semantics].size == 720
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
