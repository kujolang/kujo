# Paired same-path allocation gate for bounded row writes, including overflow.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected BASE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR [improvement|control]' unless [4, 5].include?(ARGV.size)
target = ARGV.size == 5 ? ARGV.pop : 'improvement'
raise 'Unknown target mode' unless %w[improvement control].include?(target)
baseline, candidate, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[baseline, candidate, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
FileUtils.mkdir_p(output)
report = {status: 'running', target: target, runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
          artifacts: [baseline, candidate, binary].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] },
          rows: [], semantics: [], sources: {}}
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[struct dictionary].product([32, 128, 2048], [1, 2, 3, 4, 5, 16], %w[once fresh]).each do |shape, width, writes, snapshots|
    dir = "#{output}/#{shape}-#{width}-#{writes}-#{snapshots}"
    FileUtils.mkdir_p(dir)
    row = shape == 'struct' ? 'state.row' : 'state["row"]'
    root = shape == 'struct' ? "struct State { row: array }\nmut state := State { row: range(0, #{width}) }" : "mut state := {\"row\": range(0, #{width})}"
    mod = "#{root}\nexport func snapshot() { return state }\nexport func edit() {\nmut j := 0\nwhile j < #{writes} { #{row}[j] += 1 j += 1 }\n#{row}[-#{width}] += 1\nreturn #{row}[0]\n}\n"
    File.write(dir + '/owner.kujo', mod)
    report[:sources][Digest::SHA256.hexdigest(mod)] = mod
    [16, 32].each do |calls|
      at = ->(name, index) { row.sub('state', name) + "[#{index}]" }
      saved = snapshots == 'fresh' ? "let saved := snapshot()\n" : ''
      check = snapshots == 'fresh' ? "assert_equal(#{at.call('saved', 0)}, i * 2)\nassert_equal(#{at.call('saved', writes - 1)}, #{writes == 1 ? 'i * 2' : "i + #{writes - 1}"})" : ''
      source = "from owner import edit, snapshot\nlet original := snapshot()\nmut i := 0\nwhile i < #{calls} { #{saved}assert_equal(edit(), (i + 1) * 2)\n#{check}\ni += 1 }\nlet current := snapshot()\nassert_equal(#{at.call('original', 0)}, 0)\nassert_equal(#{at.call('original', writes - 1)}, #{writes - 1})\nassert_equal(#{at.call('current', 0)}, #{calls * 2})\nassert_equal(#{at.call('current', writes - 1)}, #{writes == 1 ? calls * 2 : calls + writes - 1})\nassert_equal(#{at.call('current', writes)}, #{writes})\nprint(\"ROW BATCH PASS\")\n"
      path = dir + "/calls-#{calls}.kujo"
      File.write(path, source)
      report[:sources][Digest::SHA256.hexdigest(source)] = source
      metadata = {shape: shape, width: width, writes: writes, snapshots: snapshots, calls: calls,
                  source_sha256: Digest::SHA256.hexdigest(source), module_sha256: Digest::SHA256.hexdigest(mod)}
      %w[vm jit].each do |mode|
        pair = [baseline, candidate].map do |probe|
          stdout, stderr, status = Open3.capture3(probe, mode, path)
          raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "ROW BATCH PASS\n"
          counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
          raise stderr unless counts
          {requests: counts[1].to_i, bytes: counts[2].to_i}
        end
        report[:rows] << metadata.merge(mode: mode, baseline: pair[0], candidate: pair[1])
        save.call
      end
      %w[vm interpreter jit].each do |mode|
        args = [binary, 'run', path, '--untrusted']
        args << '--' + mode unless mode == 'vm'
        stdout, stderr, status = Open3.capture3(*args)
        raise "#{path} #{mode}: #{stdout}#{stderr}" unless status.success? && stdout == "ROW BATCH PASS\n"
        report[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
        save.call
      end
    end
  end
  raise 'Coverage' unless report[:rows].size == 288 && report[:semantics].size == 432
  report[:rows].each do |r|
    %i[requests bytes].each { |key| raise "Total regression #{r}" if r[:candidate][key] > r[:baseline][key] }
  end
  report[:deltas] = report[:rows].group_by { |r| r.values_at(:shape, :width, :writes, :snapshots, :mode) }.map do |key, rows|
    a, b = rows.sort_by { |r| r[:calls] }
    values = %i[baseline candidate].to_h do |version|
      [version, %i[requests bytes].to_h { |metric| [metric, (b[version][metric] - a[version][metric]) / 16.0] }]
    end
    %i[requests bytes].each { |metric| raise "Added-call regression #{key}" if values[:candidate][metric] > values[:baseline][metric] }
    if target == 'improvement' && key[1] >= 128 && (2..4).cover?(key[2]) && key[3] == 'once'
      raise "Missing target improvement #{key}" unless values[:candidate][:bytes] < values[:baseline][:bytes]
    end
    {shape: key[0], width: key[1], writes: key[2], snapshots: key[3], mode: key[4], **values}
  end
  report[:deltas].group_by { |r| r.values_at(:shape, :writes, :snapshots, :mode) }.each do |key, rows|
    next unless (2..4).cover?(key[1]) && key[2] == 'once'
    small, large = rows.select { |r| r[:width] >= 128 }.sort_by { |r| r[:width] }
    raise "Remaining row-width scaling #{key}" unless small[:candidate] == large[:candidate]
  end
  report[:deltas].group_by { |r| r.values_at(:shape, :width, :writes, :snapshots) }.each do |key, rows|
    raise "VM/JIT allocation disagreement #{key}" unless rows.size == 2 &&
      rows.map { |r| r.values_at(:baseline, :candidate) }.uniq.size == 1
  end
  report[:status] = 'passed'; save.call
rescue => error
  report[:status] = 'failed'; report[:error] = error.message; save.call; raise
end
