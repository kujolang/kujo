require 'json'
require 'csv'
require 'digest'
raise 'usage: BASELINE_DIRECTORY CANDIDATE_DIRECTORY NEW_REPORT' unless ARGV.size == 3
baseline, candidate, output = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(output)
expected = %w[live vm-maps compiler-maps].flat_map do |kind|
  (kind == 'compiler-maps' ? ['jit'] : %w[vm jit]).flat_map do |mode|
    [1, 8, 32].flat_map { |count| (0..2).map { |repeat| [kind, mode, count, repeat] } }
  end
end
reports = [baseline, candidate].map { |dir| JSON.parse(File.read(dir + '/report.json')) }
raise 'collector mismatch' unless reports.map { |r| r['collector_sha256'] }.uniq.size == 1
summaries = []
[baseline, candidate].zip(reports).each_with_index do |(dir, report), version|
  raise 'binary drift' unless Digest::SHA256.file(report['binary']).hexdigest == report['binary_sha256']
  identities = report['rows'].map { |r| r.values_at('kind', 'mode', 'count', 'repeat') }
  raise 'matrix identity' unless identities == expected
  report['rows'].each do |row|
    kind, mode, count, repeat = row.values_at('kind', 'mode', 'count', 'repeat')
    stem = "#{kind}-#{mode}-#{count}-#{repeat}"
    stdout = File.read(dir + '/' + stem + '.out')
    stderr = File.read(dir + '/' + stem + '.err')
    raise 'receipt integrity' unless row['exit'] == 0 &&
      Digest::SHA256.hexdigest(stdout) == row['stdout_sha256'] &&
      Digest::SHA256.hexdigest(stderr) == row['stderr_sha256']
    rss = stderr[/^\s*(\d+)\s+maximum resident set size\s*$/, 1]
    raise 'RSS mismatch' unless rss && rss.to_i == row['peak_rss_bytes']
    if kind == 'live'
      data = CSV.parse(stdout, headers: true)
      raise 'live count' unless data.size == count
      growth = data.map.with_index do |r, index|
        raise 'iteration identity' unless r['iteration'] == index.to_s && r['jit'] == (mode == 'jit').to_s
        r['after_drop_live_bytes'].to_i - r['baseline_live_bytes'].to_i
      end
    elsif kind == 'vm-maps'
      data = JSON.parse(stdout)
      raise 'VM mapping identity' unless data['vms'] == count && data['jit'] == (mode == 'jit')
      raise 'mapping controls' unless data['control_mapped'] && data['control_unmapped']
      raise 'mapping count' unless data['observations'].size == count
      growth = data['observations'].map.with_index do |r, index|
        raise 'mapping iteration' unless r['iteration'] == index
        r['after_drop_executable_bytes'] - data['baseline_executable_bytes']
      end
    else
      data = JSON.parse(stdout)
      raise 'public retention changed' unless data['compilers'] == count &&
        data['control_mapped'] && data['control_unmapped'] &&
        data['immediate_retained'] == count && data['distinct_code_pages'] == count &&
        data['retained_code_pages'] == count
      growth = nil
    end
    if growth
      if version == 1 || mode == 'vm'
        raise 'unexpected retained VM growth' unless growth.all?(&:zero?)
      else
        raise 'baseline leak not reproduced' unless growth.first > 0 &&
          growth.each_with_index.all? { |bytes, i| bytes == growth.first * (i + 1) }
      end
    end
    summaries << {version: version == 0 ? 'baseline' : 'candidate', kind: kind,
                  mode: mode, count: count, repeat: repeat, retained_growth: growth,
                  peak_rss_bytes: rss.to_i}
  end
end
File.write(output, JSON.pretty_generate({processes: summaries.size,
  baseline_binary_sha256: reports[0]['binary_sha256'], candidate_binary_sha256: reports[1]['binary_sha256'],
  collector_sha256: reports[0]['collector_sha256'], rows: summaries,
  scope: 'Matched diagnostic processes, all raw identities checked. JIT baseline linear retention must reproduce; candidate and VM-only controls must return to baseline; public compiler retention must remain. Instrumented peak RSS is reported without a memory or speed win claim.'}) + "\n")
puts '90 baseline/candidate resource processes independently verified.'
