# Allocation controls for nested writes, escapes, and competing private journals.
# Times are deliberately excluded: this gate is suitable for a loaded machine.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'

abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 4
baseline, candidate, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
FileUtils.mkdir_p(output)
report = { status: 'running', rows: [], semantics: [], identities: [baseline, candidate, binary, __FILE__].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] } }
save = -> { File.write(output + '/controls.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[struct dict].product([2, 128, 2048], %w[repeat dense other_row escape competing]).each do |shape, width, scenario|
    dir = "#{output}/#{shape}-#{width}-#{scenario}"
    FileUtils.mkdir_p(dir)
    row = shape == 'struct' ? 'state.row' : 'state["row"]'
    other = shape == 'struct' ? 'state.other' : 'state["other"]'
    root = shape == 'struct' ? "struct State { row: array, other: array }\nmut state := State { row: range(0, #{width}), other: [0] }" : "mut state := {\"row\": range(0, #{width}), \"other\": [0]}"
    extra = case scenario
            when 'dense' then "#{row}[1] += 1"
            when 'other_row' then "#{other}[0] += 1"
            when 'escape' then "let saved := state\n#{row}[0] += 1\nassert_equal(#{shape == 'struct' ? 'saved.row' : 'saved["row"]'}[0], #{row}[0] - 1)"
            when 'competing' then 'side[0] += 1'
            else ''
            end
    result = scenario == 'competing' ? 'side[0]' : "#{row}[0]"
    mod = "#{root}\nmut side := range(0, 2048)\nexport func snapshot() { return state }\nexport func side_snapshot() { return side }\nexport func edit(n) {\n mut i := 0\n while i < n { #{row}[0] += 1\n #{extra}\n i += 1 }\n return #{result}\n}\n"
    File.write(dir + '/owner.kujo', mod)
    expected = scenario == 'escape' ? 64 : 32
    old_row = shape == 'struct' ? 'old.row' : 'old["row"]'
    final_other = shape == 'struct' ? 'final.other' : 'final["other"]'
    final_row = shape == 'struct' ? 'final.row' : 'final["row"]'
    source = "from owner import snapshot, side_snapshot, edit\nlet old := snapshot()\nlet old_side := side_snapshot()\nmut i := 0\nmut result := 0\nwhile i < 32 { result = edit(1) i += 1 }\nlet final := snapshot()\nassert_equal(result, #{expected})\nassert_equal(#{old_row}[0], 0)\nassert_equal(#{old_row}[1], 1)\nassert_equal(old_side[0], 0)\nassert_equal(#{final_row}[0], #{expected})\nassert_equal(#{final_row}[1], #{scenario == 'dense' ? 33 : 1})\nassert_equal(#{final_other}[0], #{scenario == 'other_row' ? 32 : 0})\nassert_equal(side_snapshot()[0], #{scenario == 'competing' ? 32 : 0})\nprint(\"NESTED CONTROLS PASS\")\n"
    path = dir + '/main.kujo'
    File.write(path, source)
    %w[vm jit].each do |mode|
      row_result = { shape: shape, width: width, scenario: scenario, mode: mode, source_sha256: Digest::SHA256.file(path).hexdigest, module_sha256: Digest::SHA256.file(dir + '/owner.kujo').hexdigest }
      { baseline: baseline, candidate: candidate }.each do |label, probe|
        out, err, status = Open3.capture3(probe, mode, path)
        raise "#{path} #{label} #{mode}: #{out}#{err}" unless status.success? && out == "NESTED CONTROLS PASS\n"
        counts = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
        raise err unless counts
        row_result[label] = { requests: counts[1].to_i, bytes: counts[2].to_i }
      end
      report[:rows] << row_result
      save.call
    end
    %w[vm interpreter jit].each do |mode|
      args = [binary, 'run', path, '--untrusted']
      args << '--' + mode unless mode == 'vm'
      out, err, status = Open3.capture3(*args)
      raise "#{path} #{mode}: #{out}#{err}" unless status.success? && out == "NESTED CONTROLS PASS\n"
      report[:semantics] << { shape: shape, width: width, scenario: scenario, mode: mode }
      save.call
    end
  end
  raise 'Missing cases' unless report[:rows].size == 60 && report[:semantics].size == 90
  report[:regressions] = report[:rows].select { |r| r[:candidate][:bytes] > r[:baseline][:bytes] || r[:candidate][:requests] > r[:baseline][:requests] }
  raise 'Allocation regressions require review' unless report[:regressions].empty?
  report[:status] = 'passed'
  save.call
rescue => e
  report[:status] = 'failed'
  report[:error] = e.message
  save.call
  raise
end
