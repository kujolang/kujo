# Host-side allocation regression gate; all recursion and state operations are Kujo.
# Use a matching normal-release probe and runtime; allocation traffic is not speed.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
# The shared-helper route characterizes indirect reentry; it does not assert the
# direct-recursion flatness contract. Both routes retain identical semantic gates.
route = ENV.fetch('KUJO_RECURSIVE_CAPTURE_ROUTE', 'direct')
abort 'Expected direct, shared or callback route' unless %w[direct shared callback].include?(route)
abort 'Expected MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR [BASELINE_PROBE]' unless [3, 4].include?(ARGV.size)
probe, binary, root = ARGV.first(3).map { |path| File.expand_path(path) }
baseline = ARGV[3] && File.expand_path(ARGV[3])
raise 'Output directory exists' if File.exist?(root)
[probe, binary, baseline].compact.each { |path| raise 'Expected executable probe and runtime' unless File.file?(path) && File.executable?(path) }
FileUtils.mkdir_p(root)
output = root + '/report.json'
report = {runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
          probe_sha256: Digest::SHA256.file(probe).hexdigest,
          binary_sha256: Digest::SHA256.file(binary).hexdigest,
          status: 'running', route: route, fixtures: [], rows: [], semantics: []}
if baseline
  report[:baseline_probe_sha256] = Digest::SHA256.file(baseline).hexdigest
  report[:baseline_rows] = []
end
save = -> { File.write(output, JSON.pretty_generate(report) + "\n") }
save.call
begin
  [0, 128, 2048].each do |bindings|
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
    if route == 'shared'
      module_source.sub!('let nested := descend(n - 1)', 'let nested := read_bridge(n - 1)')
      module_source.sub!('let nested := step(n - 1)', 'let nested := write_bridge(n - 1)')
      module_source += "func read_bridge(n) { return descend(n) }\n"
      module_source += "func write_bridge(n) { return step(n) }\n"
    end
    bridge_source = nil
    if route == 'callback'
      bridge_source = "export func invoke(f, n) { return f(n) }\n"
      File.write(directory + '/bridge.kujo', bridge_source)
      module_source = "from bridge import invoke\n" + module_source
      module_source.sub!('let nested := descend(n - 1)', 'let nested := invoke(descend, n - 1)')
      module_source.sub!('let nested := step(n - 1)', 'let nested := invoke(step, n - 1)')
    end
    module_path = directory + '/recursive_state.kujo'
    File.write(module_path, module_source)
    [0, 1, 4].each do |depth|
      %w[read write].each do |operation|
        [16, 32].each do |calls|
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
          report[:fixtures] << meta.merge(source: source, module_source: module_source,
                                         bridge_source: bridge_source)
          %w[vm jit].each do |mode|
            out, err, status = Open3.capture3(probe, mode, path)
            raise out + err unless status.success? && out == "RECURSIVE CAPTURE PASS\n"
            counts = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
            raise err unless counts
            report[:rows] << meta.merge(mode: mode, requests: counts[1].to_i, bytes: counts[2].to_i)
            if baseline
              before_out, before_err, before_status = Open3.capture3(baseline, mode, path)
              raise before_out + before_err unless before_status.success? && before_out == "RECURSIVE CAPTURE PASS\n"
              before_counts = before_err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
              raise before_err unless before_counts
              report[:baseline_rows] << meta.merge(mode: mode, requests: before_counts[1].to_i, bytes: before_counts[2].to_i)
            end
            save.call
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
  raise 'Incomplete controls' unless report[:rows].size == 72 && report[:semantics].size == 108
  report[:deltas] = report[:rows].group_by { |r| [r[:mode], r[:operation], r[:bindings], r[:depth]] }.map do |key, rows|
    a, b = rows.sort_by { |r| r[:calls] }
    {mode: key[0], operation: key[1], bindings: key[2], depth: key[3],
     requests_per_outer_call: (b[:requests] - a[:requests]) / 16.0,
     bytes_per_outer_call: (b[:bytes] - a[:bytes]) / 16.0}
  end
# Direct global-only recursion must not materialize unrelated bindings at each
# depth. Keep both shallow and beyond-pool-depth controls.
report[:flat_depth_one_setup] = %w[vm jit].product(%w[read write]).all? do |mode, operation|
  rows = report[:deltas].select { |r| r[:mode] == mode && r[:operation] == operation && r[:depth] == 1 }
  rows.map { |r| r[:bindings] }.sort == [0, 128, 2048] &&
    rows.map { |r| [r[:requests_per_outer_call], r[:bytes_per_outer_call]] }.uniq.size == 1
end
raise 'Recursive capture setup still scales with unrelated bindings at depth one' if route == 'direct' && !report[:flat_depth_one_setup]
report[:flat_depth_four_setup] = %w[vm jit].product(%w[read write]).all? do |mode, operation|
  rows = report[:deltas].select { |r| r[:mode] == mode && r[:operation] == operation && r[:depth] == 4 }
  rows.map { |r| r[:bindings] }.sort == [0, 128, 2048] &&
    rows.map { |r| [r[:requests_per_outer_call], r[:bytes_per_outer_call]] }.uniq.size == 1
end
raise 'Recursive capture setup still scales with unrelated bindings at depth four' if route == 'direct' && !report[:flat_depth_four_setup]
if baseline
  raise 'Incomplete paired baseline' unless report[:baseline_rows].size == 72
  report[:paired_comparisons] = report[:deltas].map do |row|
    before = report[:baseline_rows].select { |r| %i[mode operation bindings depth].all? { |key| r[key] == row[key] } }.sort_by { |r| r[:calls] }
    raise 'Invalid baseline call pair' unless before.map { |r| r[:calls] } == [16, 32]
    row.merge(requests_delta: row[:requests_per_outer_call] - (before[1][:requests] - before[0][:requests]) / 16.0,
              bytes_delta: row[:bytes_per_outer_call] - (before[1][:bytes] - before[0][:bytes]) / 16.0)
  end
  report[:no_per_call_allocation_regression] = report[:paired_comparisons].all? { |r| r[:requests_delta] <= 0 && r[:bytes_delta] <= 0 }
  raise 'Recursive per-call allocation regression' unless report[:no_per_call_allocation_regression]
end
  report[:status] = 'passed'
  save.call
  puts '72 allocation and108 semantic recursive-capture controls passed'
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
