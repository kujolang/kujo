# Paired cold and over-budget allocation gate; all state operations are Kujo.
# Use a matching normal-release probe and runtime; allocation traffic is not speed.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected BASELINE_PROBE MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 4
baseline, probe, binary, root = ARGV.map { |path| File.expand_path(path) }
raise 'Output directory exists' if File.exist?(root)
[baseline, probe, binary].each { |path| raise 'Expected executable probe and runtime' unless File.file?(path) && File.executable?(path) }
FileUtils.mkdir_p(root)
output = root + '/report.json'
report = {runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
          baseline_probe_sha256: Digest::SHA256.file(baseline).hexdigest, probe_sha256: Digest::SHA256.file(probe).hexdigest,
          binary_sha256: Digest::SHA256.file(binary).hexdigest,
          status: 'running', fixtures: [], rows: [], semantics: []}
save = -> { File.write(output, JSON.pretty_generate(report) + "\n") }
save.call
begin
  [8192].each do |bindings|
    directory = root + "/fixtures/bindings-#{bindings}"
    FileUtils.mkdir_p(directory)
    module_source = "mut counter := 7\n" + (0...bindings).map { |i| "mut extra_#{i} := #{i}\n" }.join + <<~KUJO
      export func descend(n) {
        let stamp := n * 11 + 3
        if n == 0 { return counter + stamp + n }
        let nested := descend(n - 1)
        return nested + stamp + n
      }
      export func step(n) {
        counter += 1
        let stamp := n * 11 + 3
        if n == 0 { return counter + stamp + n }
        let nested := step(n - 1)
        return nested + stamp + n
      }
      export func read_counter() { return counter }
    KUJO
    module_path = directory + '/recursive_state.kujo'
    File.write(module_path, module_source)
    [0, 1, 4].each do |depth|
      %w[read write].each do |operation|
        [1, 16, 32].each do |calls|
          callee = operation == 'read' ? 'descend' : 'step'
          stamp_sum = 6 * depth * (depth + 1) + 3 * (depth + 1)
          expected = calls * (7 + stamp_sum)
          expected += (depth + 1) * calls * (calls + 1) / 2 if operation == 'write'
          final_counter = operation == 'read' ? 7 : 7 + (depth + 1) * calls
          source = <<~KUJO
            from recursive_state import #{callee}, read_counter
            mut i := 0
            mut total := 0
            while i < #{calls} { total += #{callee}(#{depth}) i += 1 }
            assert_equal(total, #{expected})
            assert_equal(read_counter(), #{final_counter})
            print("RECURSIVE CAPTURE PASS")
          KUJO
          path = directory + "/#{operation}-depth-#{depth}-calls-#{calls}.kujo"
          File.write(path, source)
          meta = {bindings: bindings, depth: depth, operation: operation, calls: calls,
                  source_sha256: Digest::SHA256.file(path).hexdigest,
                  module_sha256: Digest::SHA256.file(module_path).hexdigest}
          report[:fixtures] << meta.merge(source: source, module_source: module_source)
          %w[vm jit].each do |mode|
            {baseline: baseline, candidate: probe}.each do |build, executable|
            out, err, status = Open3.capture3(executable, mode, path)
            raise out + err unless status.success? && out == "RECURSIVE CAPTURE PASS\n"
            counts = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
            raise err unless counts
            report[:rows] << meta.merge(build: build, mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
            save.call
            end
          end
          %w[vm interpreter jit].each do |mode|
            args = [binary, 'run', path]
            args << '--' + mode unless mode == 'vm'
            out, err, status = Open3.capture3(*args)
            raise out + err unless status.success? && out == "RECURSIVE CAPTURE PASS\n"
            report[:semantics] << meta.merge(mode: mode, status: status.exitstatus)
            save.call
          end
        end
      end
    end
  end
raise 'Incomplete controls' unless report[:rows].size == 72 && report[:semantics].size == 54
report[:comparisons] = report[:rows].group_by { |r| [r[:mode], r[:operation], r[:depth], r[:calls]] }.map do |key, rows|
  before = rows.find { |r| r[:build] == :baseline }
  after = rows.find { |r| r[:build] == :candidate }
  {mode: key[0], operation: key[1], depth: key[2], calls: key[3],
   requests_delta: after[:requests] - before[:requests], bytes_delta: after[:bytes] - before[:bytes]}
end
report[:no_cold_regression] = report[:comparisons].all? { |r| r[:requests_delta] <= 0 && r[:bytes_delta] <= 0 }
raise 'Cold or oversized capture allocation regression' unless report[:no_cold_regression]
  report[:status] = 'passed'
  save.call
  puts '72 paired allocation and 54 semantic cold-capture controls passed'
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
