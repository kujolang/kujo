require 'json'
require 'digest'
require 'open3'
require 'fileutils'
require 'time'
abort 'usage: ruby compare-native-call-targets.rb BASELINE CANDIDATE OUTPUT [ROUNDS]' unless (3..4).cover?(ARGV.length)
baseline,candidate,directory=ARGV.take(3).map{|p|File.expand_path(p)}
rounds=Integer(ARGV.fetch(3,'7'))
raise 'invalid rounds' unless rounds>0 && rounds.odd? && rounds<=99
raise 'output exists' if File.exist?(directory)
frozen='.audit-evidence/checkpoint-2026-11-02/frozen/'
driver=frozen+'checkpoint-kujo'; rust=frozen+'rust-benchmarks'; goose=frozen+'goose-benchmarks'
paths=[baseline,candidate,driver,rust,goose,'benchmarks/langarena/compare_suite.kujo','benchmarks/langarena/port.kujo','benchmarks/langarena/test.json',__FILE__,'benchmarks/langarena/probes/verify-native-call-targets.rb']
files=paths.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
raise 'diagnostic environment override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
verify=lambda do
 files.each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
 processes,status=Open3.capture2('ps','-Ao','comm=')
 raise 'active build or unavailable process inventory' unless status.success? && processes.lines.none?{|l|%w[cargo rustc].include?(File.basename(l.strip))}
end
verify.call
FileUtils.mkdir_p(directory)
identity={baseline_binary:baseline,candidate_binary:candidate,rounds:rounds,files:files}
File.write(directory+'/call-target-identities.json',JSON.pretty_generate(identity)+"\n")
state={pid:Process.pid,status:'running',started_at:Time.now.utc.iso8601,commands:[],scope:'Unchanged QuickSort/MergeSort/NeuralNet test inputs; rotating local comparison, no clean-host claim'}
save=-> {File.write(directory+'/state.json',JSON.pretty_generate(state)+"\n")}
save.call
begin
 {'quick'=>'Sort::Quick','merge'=>'Sort::Merge','neural'=>'Etc::NeuralNet'}.each do |slug,name|
  verify.call
  output=directory+'/call-target-'+slug
  FileUtils.mkdir_p(output)
  argv=[driver,'run','benchmarks/langarena/compare_suite.kujo',baseline,candidate,rust,goose,output,'benchmarks/langarena/test.json',rounds.to_s,'600000',name,'resources']
  pid=Process.spawn(*argv,out:directory+"/#{slug}-driver.log",err:[:child,:out],pgroup:true)
  state[:child_pid]=pid;state[:argv]=argv;save.call
  _,result=Process.wait2(pid)
  state.delete(:child_pid)
  state[:commands]<<{argv:argv,exit:result.exitstatus,signal:result.termsig};save.call
  raise "#{name} comparison failed" unless result.success?
  verify.call
 end
 argv=['ruby','benchmarks/langarena/probes/verify-native-call-targets.rb',directory]
 pid=Process.spawn(*argv,out:directory+'/verification.log',err:[:child,:out])
 state[:child_pid]=pid;state[:argv]=argv;save.call
 _,result=Process.wait2(pid);state.delete(:child_pid)
 state[:commands]<<{argv:argv,exit:result.exitstatus,signal:result.termsig}
 raise 'comparison verification failed' unless result.success?
 state[:status]='verified-review-pending'
rescue=>error
 state[:status]='failed';state[:error]=error.message
 raise
ensure
 state[:finished_at]=Time.now.utc.iso8601;save.call
end
puts 'Target comparison verified; review paired results and independent-session/fallback gates before acceptance.'
