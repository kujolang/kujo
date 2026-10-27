# Allocation-size attribution on an already verified module matrix. No timings.
require 'json'
require 'digest'
require 'open3'
abort 'Expected ROW_PROBE VERIFIED_MODULE_GATE OUTPUT_JSON' unless ARGV.size == 3
probe, gate, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
original = JSON.parse(File.read(gate + '/module-paired.json'))
raise 'Unverified input matrix' unless original['status'] == 'passed' && original['rows'].size == 128
report = {status: 'running', probe_sha256: Digest::SHA256.file(probe).hexdigest,
          gate_sha256: Digest::SHA256.file(gate + '/module-paired.json').hexdigest,
          runner_sha256: Digest::SHA256.file(__FILE__).hexdigest, rows: []}
save = -> { File.write(output, JSON.pretty_generate(report) + "\n") }
begin
  original['rows'].each do |row|
    dir = gate + "/module-controls/#{row['shape']}-#{row['width']}"
    path = dir + "/#{row['snapshot']}-#{row['updates_per_call']}-#{row['calls']}.kujo"
    raise 'Fixture drift' unless Digest::SHA256.file(path).hexdigest == row['source_sha256']
    raise 'Module drift' unless Digest::SHA256.file(dir + '/owner_state.kujo').hexdigest == row['module_sha256']
    out, err, status = Open3.capture3(probe, row['mode'], path, 'row-buffers')
    raise out + err unless status.success? && out == "MODULE OWNER ISOLATION PASS\n"
    totals = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
    sizes = err.match(/ROW_BUFFER_AUDIT value_size=(\d+) short=(\d+) wide=(\d+)/)
    raise err unless totals && sizes
    raise 'Instrumentation changed allocation totals' unless totals[1].to_i == row['candidate']['requests'] && totals[2].to_i == row['candidate']['bytes']
    report[:rows] << row.reject { |key, _| %w[baseline candidate].include?(key) }.merge(
      requests: totals[1].to_i, bytes: totals[2].to_i, value_size: sizes[1].to_i,
      short_buffers: sizes[2].to_i, wide_buffers: sizes[3].to_i)
    save.call
  end
  report[:slopes] = report[:rows].group_by { |r| [r['shape'], r['width'], r['snapshot'], r['updates_per_call'], r['mode']] }.map do |key, rows|
    first, last = rows.sort_by { |r| r['calls'] }
    raise 'Missing call-count pair' unless rows.size == 2 && last['calls'] > first['calls']
    counter = key[1] == 128 ? :short_buffers : :wide_buffers
    buffers = last[counter] - first[counter]
    bytes = last[:bytes] - first[:bytes]
    {shape: key[0], width: key[1], snapshot: key[2], updates_per_call: key[3], mode: key[4],
     additional_calls: last['calls'] - first['calls'], bytes: bytes,
     matching_buffers: buffers, matching_buffer_bytes: buffers * key[1] * last[:value_size],
     other_bytes: bytes - buffers * key[1] * last[:value_size]}
  end
  report[:status] = 'measured'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
