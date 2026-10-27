# Diagnose successful lookup costs independently of collection-width copying.
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
save = -> { File.write(dir + '/dictionary-reads.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  %w[read write].product([8, 1024], [128, 256]).each do |operation, length, updates|
    root = dir + "/#{operation}-#{length}-#{updates}"
    FileUtils.mkdir_p(root)
    body = operation == 'read' ? 'total += state[key][1]' : 'state[key][0] += 1'
    result = operation == 'read' ? 'total' : 'state[key][0]'
    final = operation == 'read' ? 0 : updates
    state = <<~KUJO
      const key := "#{'k' * length}"
      mut state := {key: range(0, 16)}
      export func edit(n) {
        mut total := 0
        mut i := 0
        while i < n { #{body} i += 1 }
        return #{result}
      }
      export func snapshot() { return state }
      export func get_key() { return key }
    KUJO
    main = <<~KUJO
      from state import edit, snapshot, get_key
      let key := get_key()
      let old := snapshot()
      assert_equal(edit(#{updates}), #{updates})
      assert_equal(old[key][0], 0)
      assert_equal(snapshot()[key][0], #{final})
      assert_equal(snapshot()[key][15], 15)
      print("DICTIONARY READ PASS")
    KUJO
    File.write(root + '/state.kujo', state)
    path = root + '/main.kujo'
    File.write(path, main)
    metadata = {operation: operation, length: length, updates: updates,
                source_sha256: Digest::SHA256.hexdigest(main), module_sha256: Digest::SHA256.hexdigest(state)}
    %w[vm jit].each do |mode|
      row = metadata.merge(mode: mode)
      {baseline: baseline, candidate: candidate}.each do |label, probe|
        out, err, status = Open3.capture3(probe, mode, path)
        raise out + err unless status.success? && out == "DICTIONARY READ PASS\n"
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
      raise out + err unless status.success? && out == "DICTIONARY READ PASS\n"
      report[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
      save.call
    end
  end
  raise 'Missing cases' unless report[:rows].size == 16 && report[:semantics].size == 24
  report[:regressions] = report[:rows].select { |r| r[:candidate][:requests] > r[:baseline][:requests] || r[:candidate][:bytes] > r[:baseline][:bytes] }
  raise 'Allocation regression' unless report[:regressions].empty?
  report[:slopes] = %w[vm jit].product([8, 1024]).map do |mode, length|
    first, last = report[:rows].select { |r| r[:operation] == 'read' && r[:mode] == mode && r[:length] == length }.sort_by { |r| r[:updates] }
    {mode: mode, length: length, baseline_bytes: last[:baseline][:bytes] - first[:baseline][:bytes],
     candidate_bytes: last[:candidate][:bytes] - first[:candidate][:bytes],
     candidate_requests: last[:candidate][:requests] - first[:candidate][:requests]}
  end
  save.call
  %w[vm jit].each do |mode|
    short, long = report[:slopes].select { |r| r[:mode] == mode }.sort_by { |r| r[:length] }
    raise 'Baseline key-length cost not reproduced' unless long[:baseline_bytes] > short[:baseline_bytes]
    raise 'Candidate key-length allocation remains' unless long[:candidate_bytes] == short[:candidate_bytes] && long[:candidate_requests] == short[:candidate_requests]
    raise 'No allocation improvement' unless long[:candidate_bytes] < long[:baseline_bytes]
  end
  report[:status] = 'passed'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
