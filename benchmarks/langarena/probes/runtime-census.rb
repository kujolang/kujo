# Repository-maintenance collector. Uses the unmodified accepted CLI and cases.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
binary = File.realpath(ARGV.fetch(0))
root = File.expand_path(ARGV.fetch(1))
repo = File.realpath('.')
raise 'new output directory required' if File.exist?(root)
FileUtils.mkdir_p(root)
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
config_path = repo+'/benchmarks/langarena/test.json'
port = repo+'/benchmarks/langarena/port.kujo'
cases = JSON.parse(File.read(config_path))
raise 'case coverage' unless cases.size == 50 && cases.map { |c| c['name'] }.uniq.size == 50
sources = [config_path, port, File.expand_path(__FILE__), repo+'/src/benchmarks/runtime_measurements.rs'].to_h { |p| [p, sha.call(p)] }
report = {status:'running', pid:Process.pid, execution_cwd:repo, binary:binary, binary_sha256:sha.call(binary), sources:sources, rows:[], limits:'Process event counters include setup, warmup, execution and checksum. Native entries and VM calls are not CPU-time coverage or comparable denominators. Local wall/CPU/RSS observations are diagnostic, not competitive performance results.'}
write = -> { File.write(root+'/report.json.tmp', JSON.pretty_generate(report)+"\n"); File.rename(root+'/report.json.tmp',root+'/report.json') }
write.call
begin
  cases.each_with_index do |config, index|
    %w[vm jit].each do |mode|
      measurements = format('%02d-%s.json',index,mode)
      argv = [binary,'run','--scheduler-no-timeout']
      argv << '--jit' if mode == 'jit'
      argv += ['--measurements',root+'/'+measurements,port,config_path,config['name']]
      stdout, stderr, status = Open3.capture3(*argv)
      row = {name:config['name'],mode:mode,argv:argv,exit:status.exitstatus,stdout:stdout,stderr:stderr,measurement_file:measurements}
      report[:rows] << row
      write.call
      lines = stdout.lines.map(&:strip)
      raise "execution #{config['name']}/#{mode}" unless status.success? && lines.include?("#{config['name']}: OK #{config['checksum']}") && lines.include?('Summary: 1, 1, 0')
      data = JSON.parse(File.read(root+'/'+measurements))
      raise 'measurement contract' unless data['schema']=='kujo.runtime-measurements/v1' && data['outcome']=='success' && data['scope']=='process'
      row[:measurements] = data
      row[:measurement_sha256] = sha.call(root+'/'+measurements)
      write.call
    end
  end
  sources.each { |path, hash| raise 'source drift' unless sha.call(path)==hash }
  raise 'binary drift' unless sha.call(binary)==report[:binary_sha256]
  report[:status]='measured';write.call
rescue => error
  report[:status]='failed';report[:error]=error.message;write.call;raise
end
