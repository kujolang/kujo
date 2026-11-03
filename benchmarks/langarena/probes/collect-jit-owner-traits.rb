require 'json'
require 'open3'
require 'digest'
require 'fileutils'
raise 'usage: BASELINE_RLIB CANDIDATE_RLIB DEPENDENCY_DIRECTORY NEW_OUTPUT' unless ARGV.size == 4
baseline, candidate, dependencies, output = ARGV.map { |p| File.expand_path(p) }
raise 'output already exists' if File.exist?(output)
FileUtils.mkdir_p(output)
source = File.join(__dir__, 'jit_owner_traits.rs')
toolchain, error, status = Open3.capture3('rustc', '--version', '--verbose')
raise error unless status.success?
results = []
{baseline: baseline, candidate: candidate}.each do |version, library|
  %w[send sync pool].each do |trait|
    argv = ['rustc', '--edition=2021', '--emit=metadata', source, '--extern',
            "kujo=#{library}", '-L', "dependency=#{dependencies}", '-o',
            File.join(output, "#{version}-#{trait}.rmeta")]
    argv += ['--cfg', "require_#{trait}"] unless trait == 'send'
    stdout, stderr, result = Open3.capture3(*argv)
    File.write(File.join(output, "#{version}-#{trait}.stderr"), stderr)
    expected_success = trait != 'sync'
    errors = stderr.scan(/^error\[([^\]]+)\]/).flatten
    valid = result.success? == expected_success && (expected_success ||
      (!errors.empty? && errors.all? { |code| code == 'E0277' } &&
       stderr.include?('cannot be shared between threads safely') && stderr.include?('JitCompiler')))
    results << {version: version, trait: trait, argv: argv, exit: result.exitstatus,
                library_sha256: Digest::SHA256.file(library).hexdigest,
                stdout: stdout, stderr: stderr, verified: valid}
  end
end
receipt = {toolchain: toolchain, source_sha256: Digest::SHA256.file(source).hexdigest,
           scope: 'Compile-only trait checks; no execution, lifetime, performance or cross-platform proof',
           results: results}
File.write(File.join(output, 'results.json'), JSON.pretty_generate(receipt))
raise 'trait matrix failed; see preserved results' unless results.all? { |r| r[:verified] }
puts JSON.generate({verified: results.size, send: true, sync: false, mutex_pool_send_sync: true})
