# Rejected numeric-key admission must not allocate beyond the ordinary runtime.
# Timing is diagnostic only; compare matching release probes on identical paths.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'

abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 4
baseline, candidate, binary, dir = ARGV.map { |p| File.expand_path(p) }
raise 'Output directory already exists' if File.exist?(dir)
[baseline, candidate, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
FileUtils.mkdir_p(dir)
report = {status: 'running', rows: [], semantics: [],
          baseline_sha256: Digest::SHA256.file(baseline).hexdigest,
          candidate_sha256: Digest::SHA256.file(candidate).hexdigest,
          binary_sha256: Digest::SHA256.file(binary).hexdigest,
          runner_sha256: Digest::SHA256.file(__FILE__).hexdigest}
save = -> { File.write(dir + '/dictionary-keys.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[integer string].product(%w[fresh once alternate]).each do |kind, scenario|
    root = dir + '/' + kind + '-' + scenario
    FileUtils.mkdir_p(root)
    key = kind == 'integer' ? '0' : '"0"'
    other = kind == 'integer' ? '1' : '"1"'
    second = scenario == 'alternate' ? "data[#{other}] += 1" : ''
    state = <<~KUJO
      mut data := {"0": 0, "1": 0, "01": 99}
      export func edit(n) {
        mut i := 0
        while i < n { data[#{key}] += 1 #{second} i += 1 }
        return data[#{key}]
      }
      export func snapshot() { return data }
    KUJO
    saved = scenario == 'fresh' ? 'let saved := snapshot()' : ''
    check = scenario == 'fresh' ? 'assert_equal(saved["0"], i * 8)' : ''
    final_second = scenario == 'alternate' ? 256 : 0
    main = <<~KUJO
      from state import edit, snapshot
      let original := snapshot()
      mut i := 0
      while i < 32 {
        #{saved}
        assert_equal(edit(8), (i + 1) * 8)
        #{check}
        i += 1
      }
      assert_equal(original["0"], 0)
      assert_equal(original["1"], 0)
      let final := snapshot()
      assert_equal(final["0"], 256)
      assert_equal(final["1"], #{final_second})
      assert_equal(final["01"], 99)
      print("DICTIONARY KEY PASS")
    KUJO
    File.write(root + '/state.kujo', state)
    path = root + '/main.kujo'
    File.write(path, main)
    metadata = {kind: kind, scenario: scenario, source_sha256: Digest::SHA256.hexdigest(main), module_sha256: Digest::SHA256.hexdigest(state)}
    %w[vm jit].each do |mode|
      row = metadata.merge(mode: mode)
      {baseline: baseline, candidate: candidate}.each do |label, probe|
        out, err, status = Open3.capture3(probe, mode, path)
        raise out + err unless status.success? && out == "DICTIONARY KEY PASS\n"
        counts = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
        raise err unless counts
        row[label] = {requests: counts[1].to_i, bytes: counts[2].to_i}
      end
      report[:rows] << row
      save.call
    end
    %w[vm interpreter jit].each do |mode|
      args = [binary, 'run', path]
      args << '--' + mode unless mode == 'vm'
      out, err, status = Open3.capture3(*args)
      raise out + err unless status.success? && out == "DICTIONARY KEY PASS\n"
      report[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
      save.call
    end
  end
  report[:regressions] = report[:rows].select { |r| r[:candidate][:requests] > r[:baseline][:requests] || r[:candidate][:bytes] > r[:baseline][:bytes] }
  raise 'Missing cases' unless report[:rows].size == 12 && report[:semantics].size == 18
  raise 'Dictionary key allocation regression' unless report[:regressions].empty?
  report[:status] = 'passed'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
