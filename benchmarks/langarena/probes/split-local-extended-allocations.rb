# Split-frame shared-helper then foreign-module reentry; characterization only.
# Allocation traffic is the metric; elapsed timing on a busy host is not a gate.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected PROBE BINARY NEW_OUTPUT_DIR [BASELINE_PROBE]' unless [3, 4].include?(ARGV.size)
probe, binary, output = ARGV.first(3).map { |p| File.expand_path(p) }
baseline = ARGV[3] && File.expand_path(ARGV[3])
raise 'Output exists' if File.exist?(output)
[probe, binary, baseline].compact.each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
FileUtils.mkdir_p(output)
report = { status: 'running', route: 'shared-helper then foreign callback', derived_from_runner_sha256: '482049d792e09b5fa9c093bd8d6edf226565ab0cbc4bbb340e0c7c29662e5f87', runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
           probe_sha256: Digest::SHA256.file(probe).hexdigest, binary_sha256: Digest::SHA256.file(binary).hexdigest,
           baseline_probe_sha256: baseline && Digest::SHA256.file(baseline).hexdigest,
           rows: [], semantics: [], fixtures: [], modules: {} }
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  [4096, 8192, 12288, 16384].product(%w[read write]).each do |bindings, operation|
    dir = "#{output}/#{bindings}-#{operation}"
    FileUtils.mkdir_p(dir)
    declarations = (0...bindings).map { |i| "mut extra_#{i} := #{i}\n" }.join
    check = (0...bindings).map { |i| "check += extra_#{i}\n" }.join
    mutation = operation == 'write' ? 'counter += 1' : ''
    mod = "from bridge import invoke\nfunc bounce(f, n, flag) { return invoke(f, n, flag) }\nexport func factory() {\nmut counter := 7\n#{declarations}func step(n, flag) {\nif flag { mut check := 0\n#{check}assert_equal(check, #{bindings * (bindings - 1) / 2}) }\n#{mutation}\nif n > 0 { return bounce(step, n - 1, false) }\nreturn counter\n}\nreturn step\n}\n"
    module_hash = Digest::SHA256.hexdigest(mod)
    report[:modules][module_hash] = mod
    File.write(dir + '/owner.kujo', mod)
    bridge = "mut invocations := 0\nexport func invoke(f, n, flag) { invocations += 1 return f(n, flag) }\nexport func bridge_count() { return invocations }\n"
    report[:bridge_source] = bridge
    report[:bridge_sha256] = Digest::SHA256.hexdigest(bridge)
    File.write(dir + '/bridge.kujo', bridge)
    [0, 1, 4].product([1, 16, 32]).each do |depth, calls|
      increment = operation == 'write' ? depth + 1 : 0
      total = calls * 7 + increment * calls * (calls + 1) / 2
      final = 7 + increment * calls + (operation == 'write' ? 1 : 0)
      separate = operation == 'write' ? 8 : 7
      source = "from owner import factory\nfrom bridge import bridge_count\nlet chosen := factory()\nlet separate := factory()\nmut total := 0\nmut i := 0\nwhile i < #{calls} { total += chosen(#{depth}, false) i += 1 }\nassert_equal(total, #{total})\nlet alias := chosen\nassert_equal(alias(0, true), #{final})\nassert_equal(separate(0, true), #{separate})\nassert_equal(bridge_count(), #{depth * calls})\nprint(\"RECURSIVE LOCAL PASS\")\n"
      path = "#{dir}/depth-#{depth}-calls-#{calls}.kujo"
      File.write(path, source)
      metadata = { bindings: bindings, operation: operation, depth: depth, calls: calls,
                   source_sha256: Digest::SHA256.hexdigest(source), module_sha256: module_hash }
      report[:fixtures] << metadata.merge(source: source)
      %w[vm jit].each do |mode|
        row = metadata.merge(mode: mode)
        { candidate: probe, baseline: baseline }.each do |label, executable|
          next unless executable
          out, err, status = Open3.capture3(executable, mode, path)
          raise "#{path} #{mode}: #{out}#{err}" unless status.success? && out == "RECURSIVE LOCAL PASS\n"
          counts = err.match(/requests=(\d+) requested_bytes=(\d+)/)
          raise err unless counts
          row[label] = { requests: counts[1].to_i, bytes: counts[2].to_i }
        end
        report[:rows] << row
        save.call
      end
      %w[vm interpreter jit].each do |mode|
        args = [binary, 'run', path, '--untrusted']
        args << '--' + mode unless mode == 'vm'
        out, err, status = Open3.capture3(*args)
        raise "#{path} #{mode}: #{out}#{err}" unless status.success? && out == "RECURSIVE LOCAL PASS\n"
        report[:semantics] << metadata.merge(mode: mode, exit: status.exitstatus)
        save.call
      end
    end
  end
  raise 'Missing coverage' unless report[:rows].size == 144 && report[:semantics].size == 216
  report[:deltas] = report[:rows].group_by { |r| r.values_at(:bindings, :operation, :depth, :mode) }.map do |key, rows|
    first, last = rows.select { |r| [16, 32].include?(r[:calls]) }.sort_by { |r| r[:calls] }
    delta = { bindings: key[0], operation: key[1], depth: key[2], mode: key[3] }
    [:candidate, :baseline].each do |label|
      next unless first[label]
      delta[label] = { requests: (last[label][:requests] - first[label][:requests]) / 16.0,
                       bytes: (last[label][:bytes] - first[label][:bytes]) / 16.0 }
    end
    delta
  end
  if baseline
    report[:regressions] = (report[:rows] + report[:deltas]).select { |r| r[:candidate][:bytes] > r[:baseline][:bytes] || r[:candidate][:requests] > r[:baseline][:requests] }
    save.call
    raise 'Allocation regressions require review' unless report[:regressions].empty?
    # This runner reports the remaining cost; it does not accept an optimization.
  end
  report[:status] = baseline ? 'compared_without_regressions' : 'characterized'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
