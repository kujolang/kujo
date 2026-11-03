# Independent review of the source-pinned Linux all150 artifact, including CLI bytes.
require 'json'
require 'digest'
require 'open3'
require 'stringio'
raise 'usage: ARTIFACT_DIRECTORY COMMIT' unless ARGV.size==2
dir,commit=ARGV
raise 'full commit' unless commit.match?(/\A[0-9a-f]{40}\z/)
read=->(name){File.binread(File.join(dir,name))}
run=JSON.parse(read.call('run.json'));jobs=JSON.parse(read.call('jobs.json'))['jobs'];artifact=JSON.parse(read.call('artifact.json'))
raise 'run provenance' unless run['head_sha']==commit && run['conclusion']=='success' && run.dig('repository','full_name')=='kujolang/kujo'
raise 'artifact provenance' unless artifact.dig('workflow_run','id')==run['id'] && artifact.dig('workflow_run','head_sha')==commit && artifact['name']=='langarena-all150-linux'
workflow,err,status=Open3.capture3('git','show',"#{commit}:.github/workflows/vm-code-lifetime.yml");raise err unless status.success?
expected_jobs=[['checksum / all150','success'],['lifetime','skipped']]
expected_jobs<<['elapsed','skipped'] if workflow.include?("\n  elapsed:\n")
expected_jobs<<['profile','skipped'] if workflow.include?("\n  profile:\n")
raise 'job membership/outcome' unless jobs.map{|j|[j['name'],j['conclusion']]}.sort==expected_jobs.sort
source=read.call('source.txt')
raise 'source/host' unless source.lines.first.strip==commit && source.lines.map(&:strip).include?('host: x86_64-unknown-linux-gnu')
manifest=read.call('source-sha256.txt').lines.map do |line|
 match=line.match(/\A([0-9a-f]{64})  ([^\n]+)\n\z/);raise 'source manifest format' unless match
 [match[2],match[1]]
end
paths,err,status=Open3.capture3('git','ls-tree','-r','--name-only',commit,'--','src','tests','Cargo.toml','Cargo.lock','benchmarks/langarena/src','benchmarks/langarena/test.json','benchmarks/langarena/probes/calculator-accumulator-regression.kujo')
raise err unless status.success?
raise 'source coverage' unless manifest.map(&:first)==paths.lines.map(&:strip) && manifest.map(&:first).uniq.size==manifest.size
blob,err,status=Open3.capture3('git','cat-file','--batch',stdin_data:manifest.map{|p,_|"#{commit}:#{p}\n"}.join)
raise err unless status.success?
stream=StringIO.new(blob)
manifest.each do |path,hash|
 header=stream.gets;match=header&.match(/\A[0-9a-f]+ blob (\d+)\n\z/);raise 'blob header' unless match
 bytes=stream.read(match[1].to_i);raise "source hash #{path}" unless Digest::SHA256.hexdigest(bytes)==hash && stream.read(1)=="\n"
end
raise 'extra source bytes' unless stream.eof?
raise 'source recheck' unless read.call('source-recheck.log').lines.map(&:strip)==manifest.map{|p,_|p+': OK'}
binary_line=read.call('binary-sha256.txt').match(/\A([0-9a-f]{64})  checksum-evidence\/kujo\n\z/)
raise 'binary identity' unless binary_line && Digest::SHA256.file(File.join(dir,'kujo')).hexdigest==binary_line[1]
raise 'binary recheck' unless read.call('binary-recheck.log')=="checksum-evidence/kujo: OK\n"
messages=read.call('build.jsonl').lines.map{|line|JSON.parse(line)}
raise 'build outcome' unless messages.select{|m|m['reason']=='build-finished'}.map{|m|m['success']}==[true]
%w[kujo langarena_port_contracts].each do |name|
 entries=messages.select{|m|m['reason']=='compiler-artifact' && m.dig('target','name')==name && m['executable']}
 raise 'JIT feature coverage' unless entries.size == 1 && entries[0].fetch('features', []).include?('runtime-jit')
 raise 'optimized build coverage' unless entries.size==1 && entries[0].dig('profile','opt_level')=='3' && entries[0].dig('profile','debug_assertions')==false && entries[0].dig('profile','test')==(name=='langarena_port_contracts')
end
config,err,status=Open3.capture3('git','show',"#{commit}:benchmarks/langarena/test.json");raise err unless status.success?
cases=JSON.parse(config);raise 'registry' unless cases.size==50 && cases.map{|c|c['name']}.uniq.size==50
expected=cases.map{|c|"#{c.fetch('name')}: OK #{c.fetch('checksum')}"}
log=read.call('checksums.log').gsub(/\e\[[0-9;]*m/,'')
raise 'runtime coverage' unless log.scan(/^Runtime: (.+)$/).flatten==%w[vm interpreter jit]
sections=log.split(/^Runtime: /)[1..-1]
sections.zip(%w[vm interpreter jit]).each do |section,mode|
 lines=section.lines.map(&:strip)
 raise "checksum membership #{mode}" unless lines.select{|l|l.include?(': OK ')}==expected
 raise "summary #{mode}" unless lines.count('Summary: 50, 50, 0')==1
end
raise 'test membership' unless log.scan(/^test (.+) \.\.\. ok$/).flatten==['all_fifty_upstream_checksums_in_all_runtimes']
test_source,test_error,test_status=Open3.capture3('git','show',"#{commit}:tests/langarena_port_contracts.rs")
raise test_error unless test_status.success?
filtered=(test_source.scan(/^#\[test\]/).size-1).to_s
raise 'test result' unless log.scan(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/)==[['1','0','0','0',filtered]]
result={status:'verified',commit:commit,run:run['id'],artifact:artifact['id'],binary_sha256:binary_line[1],source_files:manifest.size,checksum_outcomes:150,runtimes:%w[vm interpreter jit],scope:'Exact test-size checksum evidence; no production, native-coverage, timing or final optimization acceptance claim.'}
result[:evidence_sha256]=%w[source.txt source-sha256.txt source-recheck.log binary-sha256.txt binary-recheck.log build.jsonl build.stderr checksums.log run.json jobs.json artifact.json].to_h{|f|[f,Digest::SHA256.hexdigest(read.call(f))]}
regression='calculator_accumulator_wraps_before_conversion_in_all_runtimes'
if test_source.include?("fn #{regression}()")
 regression_log=read.call('calculator-regression.log').gsub(/\e\[[0-9;]*m/,'')
 raise 'calculator regression membership' unless regression_log.scan(/^test (.+) \.\.\. ok$/).flatten==[regression]
 raise 'calculator regression result' unless regression_log.scan(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/)==[['1','0','0','0',filtered]]
 result[:calculator_regression]='passed-all-three-runtimes'
 result[:evidence_sha256]['calculator-regression.log']=Digest::SHA256.hexdigest(read.call('calculator-regression.log'))
end
File.write(File.join(dir,'verification.json'),JSON.pretty_generate(result)+"\n")
puts JSON.generate(result.reject{|k,_|k==:evidence_sha256})
