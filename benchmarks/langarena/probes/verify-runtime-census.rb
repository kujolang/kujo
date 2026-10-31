require 'json'
require 'digest'
root=File.expand_path(ARGV.fetch(0));sha=->(p){Digest::SHA256.file(p).hexdigest}
r=JSON.parse(File.read(root+'/report.json'));raise 'unfinished' unless r['status']=='measured'
raise 'binary drift' unless sha.call(r['binary'])==r['binary_sha256']
r['sources'].each{|p,h|raise 'source drift' unless sha.call(p)==h}
repo=r['execution_cwd'];config_path=repo+'/benchmarks/langarena/test.json';port=repo+'/benchmarks/langarena/port.kujo'
cases=JSON.parse(File.read(config_path));expected=cases.flat_map{|c|%w[vm jit].map{|m|[c['name'],m]}}
raise 'coverage' unless expected.size==100&&r['rows'].map{|x|x.values_at('name','mode')}==expected
names=File.read(repo+'/src/benchmarks/runtime_measurements.rs').scan(/=> "([a-z_]+)"/).flatten.sort
raise 'metric inventory' unless names.size>30&&names.uniq==names
summary=[]
r['rows'].each_with_index do |row,i|
  config=cases[i/2];mode=row['mode'];file=format('%02d-%s.json',i/2,mode)
  argv=[r['binary'],'run','--scheduler-no-timeout'];argv<<'--jit' if mode=='jit';argv+=['--measurements',root+'/'+file,port,config_path,config['name']]
  raise 'argv/file' unless row['argv']==argv&&row['measurement_file']==file
  lines=row['stdout'].lines.map(&:strip)
  raise 'checksum' unless row['exit']==0&&lines.count("#{config['name']}: OK #{config['checksum']}")==1&&lines.count('Summary: 1, 1, 0')==1
  d=JSON.parse(File.read(root+'/'+file));raise 'receipt' unless d==row['measurements']&&sha.call(root+'/'+file)==row['measurement_sha256']
  raise 'contract' unless d.values_at('schema','scope','outcome')==['kujo.runtime-measurements/v1','process','success']
  c=d['counters'];raise 'counter inventory' unless c.keys.sort==names&&c.values.all?{|v|v.is_a?(Integer)&&v>=0}
  summary<<{name:row['name'],mode:mode,counters:c.reject{|k,_|k.end_with?('_ns')},diagnostic_only:{wall_ns:d['wall_ns'],cpu_seconds:d['cpu_seconds'],peak_rss_bytes:d['process_peak_rss_bytes'],jit_compile_inclusive_wall_ns:c['jit_compile_inclusive_wall_ns']}}
end
receipt={status:'passed',checksum_runs:100,report_sha256:sha.call(root+'/report.json'),binary_sha256:r['binary_sha256'],rows:summary,limits:r['limits']}
File.write(root+'/verified.json',JSON.pretty_generate(receipt)+"\n");puts '100 VM/JIT checksum runs and exact metric inventories verified'
