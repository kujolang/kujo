# Repository-maintenance diagnostic: pin unchanged port/config, inject only argv.
# Counts execution-time requested allocation traffic, not retained memory or speed.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
abort 'Expected BASELINE_PROBE CANDIDATE_PROBE CANDIDATE_BINARY NEW_OUTPUT_DIR' unless ARGV.size == 4
baseline, candidate, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'Output exists' if File.exist?(output)
[baseline, candidate, binary].each { |p| raise 'Missing executable ' + p unless File.executable?(p) }
port_path = File.expand_path('benchmarks/langarena/port.kujo')
config_path = File.expand_path('benchmarks/langarena/test.json')
port = File.read(port_path)
config = JSON.parse(File.read(config_path))
names = %w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::CacheSimulation Etc::GameOfLife Etc::TextRaytracer Brainfuck::Array Brainfuck::Recursion]
raise 'Argument injection boundary changed' unless port.scan('let arguments := args()').size == 1
raise 'Missing/duplicate priority cases' unless names.all? { |n| config.count { |c| c['name'] == n } == 1 }
FileUtils.mkdir_p(output)
r = { status: 'running', pid: Process.pid, scope: 'Eight unchanged priority test-size workloads; argv injection only; execution allocations include setup/warmup/checksum',
      runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
      identities: [baseline, candidate, binary, port_path, config_path].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] },
      rows: [], semantics: [], fixtures: [] }
save = -> { File.write(output + '/report.json', JSON.pretty_generate(r) + "\n") }
save.call
begin
  names.each_with_index do |name, index|
    configuration = config.find { |c| c['name'] == name }
    source = port.sub('let arguments := args()', 'let arguments := ' + JSON.generate([config_path, name, '--profile']))
    path = output + '/' + name.gsub('::', '-') + '.kujo'
    File.write(path, source)
    metadata = { name: name, configuration: configuration, path: path, source_sha256: Digest::SHA256.hexdigest(source) }
    r[:fixtures] << metadata
    expected = name + ': OK ' + configuration.fetch('checksum').to_s
    %w[vm jit].each_with_index do |mode, mode_index|
      row = metadata.merge(mode: mode)
      # Alternate revision order; no timing acceptance is derived from this run.
      order = (index + mode_index).even? ? %i[baseline candidate] : %i[candidate baseline]
      row[:order] = order
      order.each do |label|
        executable = label == :baseline ? baseline : candidate
        out, err, status = Open3.capture3(executable, mode, path)
        File.write(output + "/#{name.gsub('::', '-')}-#{mode}-#{label}.stdout", out)
        File.write(output + "/#{name.gsub('::', '-')}-#{mode}-#{label}.stderr", err)
        raise "Checksum/execution failure #{name} #{mode} #{label}: #{out}#{err}" unless status.success? && out.lines.map(&:strip).include?(expected) && out.include?('Summary: 1, 1, 0')
        counts = err.match(/ALLOCATION_AUDIT requests=(\d+) requested_bytes=(\d+)/)
        raise 'Missing allocation observation' unless counts
        row[label] = { requests: counts[1].to_i, bytes: counts[2].to_i, exit: status.exitstatus, stdout: out, stderr: err }
      end
      r[:rows] << row
      save.call
    end
    out, err, status = Open3.capture3(binary, 'run', path, '--interpreter', '--scheduler-no-timeout')
    raise "Interpreter checksum failure #{name}: #{out}#{err}" unless status.success? && out.lines.map(&:strip).include?(expected) && out.include?('Summary: 1, 1, 0')
    r[:semantics] << metadata.merge(mode: 'interpreter', exit: status.exitstatus, stdout: out, stderr: err)
    save.call
  end
  raise 'Allocation coverage' unless r[:rows].map { |x| x.values_at(:name, :mode) }.sort == names.product(%w[vm jit]).sort
  raise 'Interpreter coverage' unless r[:semantics].map { |x| x[:name] } == names
  r[:identities].each { |p, h| raise 'Artifact/source drift ' + p unless Digest::SHA256.file(p).hexdigest == h }
  r[:fixtures].each { |f| raise 'Fixture drift' unless Digest::SHA256.file(f[:path]).hexdigest == f[:source_sha256] }
  r[:status] = 'characterized'
  save.call
rescue => e
  r[:status] = 'failed'
  r[:error] = e.message
  save.call
  raise
end
