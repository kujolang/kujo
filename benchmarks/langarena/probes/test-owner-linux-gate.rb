require 'tmpdir'
require 'fileutils'
require 'json'
require 'open3'
raise 'usage: VERIFIED_DIRECTORY FULL_COMMIT' unless ARGV.size == 2
source, commit = ARGV
verifier = File.join(__dir__, 'verify-owner-linux-gate.rb')
results = []
Dir.mktmpdir('owner-linux-verifier-') do |directory|
  originals = Dir[File.join(source,'*')].select { |path| File.file?(path) }.to_h { |path| [File.basename(path),File.binread(path)] }
  controls = {'unaltered'=>nil,'wrong-source'=>'artifact source','failed-job'=>'job coverage/result','duplicate-owner'=>'owner membership','duplicate-integration'=>'pinned integration membership','filtered'=>'test summaries'}
  controls.each do |mode, error|
    originals.each { |name,bytes| File.binwrite(File.join(directory,name),bytes) }
    case mode
    when 'wrong-source'
      path=File.join(directory,'loop-condition-source.txt'); File.write(path,File.read(path).sub(commit,'0'*40))
    when 'failed-job'
      path=File.join(directory,'jobs.json'); j=JSON.parse(File.read(path)); j['jobs'][0]['conclusion']='failure'; File.write(path,JSON.generate(j))
    when 'duplicate-owner'
      path=File.join(directory,'loop-condition-tests.log'); s=File.read(path); raise 'missing fixture' unless s.sub!('test vm::code_owner_lifetime_tests::exclusive_code_owner_survives_vm_and_worker_transfer ... ok','test vm::code_owner_lifetime_tests::disabling_jit_does_not_destroy_cached_code ... ok'); File.write(path,s)
    when 'duplicate-integration'
      path=File.join(directory,'loop-condition-tests.log'); s=File.read(path); raise 'missing fixture' unless s.sub!('test failed_compiled_callback_bridge_preserves_message_and_can_be_reused ... ok','test compiled_callback_errors_stop_execution_and_reach_handlers ... ok'); File.write(path,s)
    when 'filtered'
      path=File.join(directory,'loop-condition-tests.log'); s=File.read(path); raise 'missing fixture' unless s.sub!('0 filtered out;','1 filtered out;'); File.write(path,s)
    end
    stdout, stderr, status = Open3.capture3('ruby',verifier,directory,commit)
    raise "unexpected #{mode}: #{stdout} #{stderr}" unless status.success? == error.nil?
    raise "wrong rejection #{mode}: #{stderr}" if error && !stderr.include?(error)
    results << {control:mode,exit:status.exitstatus,expected_rejection:error}
  end
end
puts JSON.pretty_generate(results)
