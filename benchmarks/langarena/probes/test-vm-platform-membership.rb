# Regression controls for the pinned platform-artifact verifier; no builds needed.
require 'tmpdir'
require 'fileutils'
require 'open3'
require 'json'
raise 'usage: VERIFIED_ARTIFACT_DIRECTORY PLATFORM COMMIT' unless ARGV.size == 3
source, platform, commit = ARGV
verifier = File.join(__dir__, 'verify-vm-platform-lifetime.rb')
results = []
Dir.mktmpdir('kujo-platform-membership-') do |directory|
  files = %w[lifetime-source.txt lifetime-library.log lifetime-integration.log]
  originals = files.to_h { |name| [name, File.binread(File.join(source, name))] }
  target = 'test vm_generator_resumes_after_its_original_native_code_owner_is_dropped ... ok'
  raise 'missing owner test fixture' unless originals.fetch('lifetime-integration.log').scan(target).size == 1
  %w[unaltered crlf duplicate fabricated].each do |mode|
    originals.each do |name, raw|
      bytes = raw.dup
      if mode == 'crlf'
        bytes = bytes.gsub("\r\n", "\n").gsub("\n", "\r\n")
      elsif name == 'lifetime-integration.log' && mode == 'duplicate'
        bytes = bytes.sub(target, 'test vm_generator_runs_nested_calls_and_all_loop_iterations ... ok')
      elsif name == 'lifetime-integration.log' && mode == 'fabricated'
        bytes = bytes.sub(target, 'test nonexistent_owner_survival_test ... ok')
      end
      File.binwrite(File.join(directory, name), bytes)
    end
    out, err, status = Open3.capture3('ruby', verifier, directory, platform, commit)
    expected = %w[unaltered crlf].include?(mode)
    raise "unexpected #{mode} result: #{out} #{err}" unless status.success? == expected
    raise "wrong #{mode} rejection: #{err}" unless expected || err.include?('pinned integration membership')
    results << {control: mode, exit: status.exitstatus, expected_acceptance: expected}
  end
end
puts JSON.pretty_generate({platform: platform, commit: commit, controls: results})
