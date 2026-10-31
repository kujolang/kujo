require 'json';require 'digest';require 'open3'
root=File.expand_path(ARGV.fetch(0));sha=->(p){Digest::SHA256.file(p).hexdigest};r=JSON.parse(File.read(root+'/report.json'))
raise 'unfinished' unless r['status']=='measured';raise 'binary' unless sha.call(r['binary'])==r['binary_sha256']
r['sources'].each{|p,h|raise 'source drift' unless sha.call(p)==h}
config_path=r['sources'].keys.find{|p|p.end_with?('/test.json')};port=r['sources'].keys.find{|p|p.end_with?('/port.kujo')};cases=JSON.parse(File.read(config_path))
expected=%w[Sort::Quick Sort::Merge Etc::NeuralNet].flat_map{|n|%w[vm jit].map{|m|[n,m]}}
raise 'coverage' unless r['rows'].map{|x|x.values_at('name','mode')}==expected
parent=File.dirname(root);build=JSON.parse(File.read(parent+'/build-state.json'));identity=JSON.parse(File.read(parent+'/rebuild-identity.json'))
raise 'build identity' unless build['status']=='built'&&build['binary_sha256']==r['binary_sha256']
build['sources'].merge(identity['source_manifest']).each{|p,h|raise 'rebuild source drift' unless sha.call(p)==h}
symbols,status=Open3.capture2('nm','-nm',r['binary']);raise 'symbol receipt' unless status.success?&&symbols==File.read(parent+'/symbols.txt')&&symbols.include?('kujo')&&symbols.include?('execute')
summaries=r['rows'].map do |row|
 cfg=cases.find{|c|c['name']==row['name']};argv=[r['binary'],row['mode'],row['name']]
 raise 'argv' unless row['argv']==argv
 lines=row['stdout'].lines.map(&:strip);raise 'checksum' unless row['exit']==0&&lines.count("#{cfg['name']}: OK #{cfg['checksum']}")==1&&lines.count('Summary: 1, 1, 0')==1
 sample=row['sample'];raise 'trigger' unless sample&&sample['trigger_line'].start_with?('@profile warmup-done ns=')&&lines.count(sample['trigger_line'])==1
 expected_file=row['mode']+'-'+row['name'].gsub('::','-')+'.sample.txt'
 raise 'sample identity' unless row['sample_file']==expected_file&&sha.call(root+'/'+expected_file)==row['sample_sha256']
 command=sample['argv'];raise 'sample invocation' unless command[0]=='/usr/bin/sample'&&command[1].match?(/\A\d+\z/)&&command.drop(2)==['3','2','-mayDie','-file',root+'/'+expected_file]
 text=File.read(root+'/'+expected_file);raise 'sampler failed' unless sample['exit']==0&&text.include?('Call graph:')&&text.include?("pid #{command[1]}")
 top=text.split('Sort by top of stack, same collapsed (when >= 5):')[1].to_s.split('Binary Images:')[0]
 {name:row['name'],mode:row['mode'],sample_sha256:row['sample_sha256'],top_stack:top.lines.map(&:strip).reject(&:empty?),resolved_runtime_functions:text.lines.map{|line|line[/((?:kujo::|_ZN4kujo)[^ ]+)/,1]}.compact.uniq}
end
raise 'missing resolved runtime functions' unless summaries.all?{|row|!row[:resolved_runtime_functions].empty?}
File.write(root+'/verified.json',JSON.pretty_generate({status:'passed',verifier_sha256:sha.call(File.expand_path(__FILE__)),checksum_runs:6,sampled_runs:6,report_sha256:sha.call(root+'/report.json'),rows:summaries,limits:r['limits']+' Collapsed totals include waiting threads; never interpret their share as CPU time. A separately linked diagnostic is not the accepted CLI artifact; generated JIT frames can remain unresolved.'})+"\n")
puts 'Six symbolized-probe checksum-bearing samples verified'
