# Host-side allocation regression gate; all exercised helpers/loops are Kujo.
# Usage: ruby imported-helper-allocations.rb MATCHING_ALLOCATION_PROBE NEW_OUTPUT_DIR
# Compile captured-append-scaling/allocation_probe.rs against the intended normal
# release library. This runner does not build, select or silently replace it.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'

abort 'Expected ALLOCATION_PROBE NEW_OUTPUT_DIR' unless ARGV.length == 2
probe = File.expand_path(ARGV.fetch(0))
output = File.expand_path(ARGV.fetch(1))
abort 'Probe must be executable' unless File.file?(probe) && File.executable?(probe)
abort 'Output directory already exists' if File.exist?(output)
fixtures_path = File.expand_path('../import-environment-attribution-2026-10-25.json', __dir__)
baseline_path = File.expand_path('../import-cost-refresh-2026-10-26.json', __dir__)
fixtures = JSON.parse(File.read(fixtures_path)).fetch('rows')
cases = JSON.parse(File.read(baseline_path)).fetch('calls').fetch('rows')
FileUtils.mkdir_p(output)
report = {
  scope: 'Whole VM execution allocation traffic, including imports; parsing/compilation excluded. Not elapsed speed or retained heap.',
  probe_sha256: Digest::SHA256.file(probe).hexdigest,
  runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
  fixture_report_sha256: Digest::SHA256.file(fixtures_path).hexdigest,
  case_report_sha256: Digest::SHA256.file(baseline_path).hexdigest,
  status: 'running', rows: []
}
save = -> { File.write(output + '/report.json', JSON.pretty_generate(report) + "\n") }
save.call
begin
  cases.each do |row|
    fixture = fixtures.find { |f| f['bindings'] == row['bindings'] && f['width'] == row['width'] }
    raise 'Missing module fixture' unless fixture
    directory = output + "/bindings-#{row['bindings']}-width-#{row['width']}"
    FileUtils.mkdir_p(directory)
    module_path = directory + '/import_probe.kujo'
    source_path = directory + "/#{row['operation']}-#{row['calls']}.kujo"
    File.write(module_path, fixture.fetch('module_source'))
    File.write(source_path, row.fetch('source'))
    raise 'Module fixture drift' unless Digest::SHA256.file(module_path).hexdigest == row.fetch('module_sha256')
    raise 'Call fixture drift' unless Digest::SHA256.file(source_path).hexdigest == row.fetch('source_sha256')
    stdout, stderr, status = Open3.capture3(probe, row.fetch('mode'), source_path)
    raise "Runtime failure: #{stdout} #{stderr}" unless status.success? && stdout == "PASS\n"
    counts = stderr.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
    raise "Missing allocation counts: #{stderr}" unless counts
    report[:rows] << row.merge('requests' => counts[1].to_i, 'bytes' => counts[2].to_i,
                              'stdout' => stdout, 'stderr' => stderr)
    save.call
  end
  raise 'Expected32 controls' unless report[:rows].length == 32
  groups = report[:rows].group_by { |r| [r['mode'], r['operation'], r['bindings'], r['width']] }
  deltas = groups.map do |key, rows|
    first, last = rows.sort_by { |r| r.fetch('calls') }
    raise 'Expected64/128 calls' unless rows.length == 2 && [first['calls'], last['calls']] == [64, 128]
    { mode: key[0], operation: key[1], bindings: key[2], width: key[3],
      added_calls: 64, requests: last['requests'] - first['requests'], bytes: last['bytes'] - first['bytes'] }
  end
  report[:deltas] = deltas
  # Compare exact integer64-call deltas. No absolute allocator-count threshold,
  # VM/JIT equality assumption, or assertion that the still-open write cost is flat.
  report[:flat_read_setup] = %w[vm jit].all? do |mode|
    reads = deltas.select { |r| r[:mode] == mode && r[:operation] == 'read' }
    reads.length == 4 && reads.map { |r| [r[:requests], r[:bytes]] }.uniq.length == 1
  end
  raise 'Read-only helper setup still scales with unrelated module bindings or array width' unless report[:flat_read_setup]
  report[:status] = 'passed'
  save.call
  puts '32 allocation/semantic controls passed; read-only setup is flat in VM and JIT'
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  warn error.message
  exit 1
end
