# macOS lifecycle diagnostics. RSS is instrumented process peak, not live heap.
require 'json'
require 'csv'
require 'digest'
require 'open3'
raise 'usage: RESOURCE_PROBE NEW_OUTPUT' unless ARGV.size == 2
binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(output)
Dir.mkdir(output)
hash = Digest::SHA256.file(binary).hexdigest
rows = []
%w[live vm-maps compiler-maps].each do |kind|
  modes = kind == 'compiler-maps' ? ['jit'] : %w[vm jit]
  modes.each do |mode|
    [1, 8, 32].each do |count|
      3.times do |repeat|
        name = "#{kind}-#{mode}-#{count}-#{repeat}"
        argv = ['/usr/bin/time', '-l', binary, count.to_s, mode, kind]
        stdout, stderr, status = Open3.capture3(*argv)
        File.write(output + '/' + name + '.out', stdout)
        File.write(output + '/' + name + '.err', stderr)
        raise "failed #{name}: #{status.exitstatus}" unless status.success?
        rss = stderr[/^\s*(\d+)\s+maximum resident set size\s*$/, 1]
        raise 'missing macOS RSS receipt' unless rss
        if kind == 'live'
          data = CSV.parse(stdout, headers: true)
          expected_headers = %w[iteration jit baseline_live_bytes before_live_bytes vm_live_bytes after_drop_live_bytes]
          raise 'live shape' unless data.headers == expected_headers && data.size == count
          data.each_with_index do |r, i|
            raise 'case identity' unless r['iteration'] == i.to_s && r['jit'] == (mode == 'jit').to_s
            expected_headers.reject { |k| k == 'jit' }.each do |k|
              raise 'invalid integer' unless r[k].match?(/\A\d+\z/)
            end
          end
          summary = {after_drop_growth_bytes: data[-1]['after_drop_live_bytes'].to_i - data[0]['baseline_live_bytes'].to_i}
        elsif kind == 'vm-maps'
          data = JSON.parse(stdout)
          raise 'mapping shape' unless data['vms'] == count && data['jit'] == (mode == 'jit') && data['observations'].size == count
          raise 'mapping control' unless data['control_mapped'] && data['control_unmapped']
          summary = {after_drop_executable_growth_bytes: data['observations'][-1]['after_drop_executable_bytes'] - data['baseline_executable_bytes']}
        else
          data = JSON.parse(stdout)
          raise 'public compiler shape' unless data['compilers'] == count
          raise 'mapping control' unless data['control_mapped'] && data['control_unmapped']
          summary = {immediate_retained: data['immediate_retained'], retained_code_pages: data['retained_code_pages']}
        end
        rows << {kind: kind, mode: mode, count: count, repeat: repeat, argv: argv,
                 exit: status.exitstatus, peak_rss_bytes: rss.to_i, summary: summary,
                 stdout_sha256: Digest::SHA256.hexdigest(stdout), stderr_sha256: Digest::SHA256.hexdigest(stderr)}
      end
    end
  end
end
raise 'binary changed' unless Digest::SHA256.file(binary).hexdigest == hash
raise 'coverage' unless rows.size == 45
File.write(output + '/report.json', JSON.pretty_generate({binary: binary, binary_sha256: hash,
  collector_sha256: Digest::SHA256.file(__FILE__).hexdigest, rows: rows,
  scope: '45 diagnostic process runs. Live requested bytes, executable virtual mappings and instrumented peak RSS are distinct. No speed claim or general platform guarantee.'}) + "\n")
puts '45 resource process outcomes collected and checked.'
