# Isolated Linux resource observations, not elapsed-speed comparisons.
require 'json'
require 'digest'
raise 'usage: OUTPUT_WITH_BUILD_RECEIPT' unless ARGV.size==1
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
dir=ARGV.first
rows=File.readlines(dir+'/build.jsonl').filter_map{|line| JSON.parse(line) rescue nil}
paths=rows.select{|row|row['reason']=='compiler-artifact' && row.dig('profile','test') && row['executable']}.map{|row|row['executable']}.uniq
raise 'test binary identity' unless paths.size==1
binary=paths.first; hash=Digest::SHA256.file(binary).hexdigest
File.write(dir+'/executable.json',JSON.generate({path:binary,sha256:hash}))
cases=[]
%w[pooled unpooled].product(%w[same distinct],[32,1024,4096]).each do |mode,shape,count|
  raise 'binary drift' unless Digest::SHA256.file(binary).hexdigest==hash
  env={'KUJO_OWNER_RESOURCE_MODE'=>mode,'KUJO_OWNER_RESOURCE_SHAPE'=>shape,'KUJO_OWNER_RESOURCE_COUNT'=>count.to_s}
  path=dir+"/#{mode}-#{shape}-#{count}.log"
  argv=['timeout','180',binary,'--exact','vm::task_owner_resource_tests::task_owner_resource_diagnostic','--ignored','--nocapture','--test-threads=1']
  passed=system(env,*argv,out:path,err:[:child,:out])
  status=$?
  cases<<{mode:mode,shape:shape,count:count,environment:env,argv:argv,exit:status.exitstatus,signal:status.termsig,log_sha256:Digest::SHA256.file(path).hexdigest}
  File.write(dir+'/cases.json',JSON.pretty_generate(cases))
  raise "resource case failed or timed out: #{path}, #{status.exitstatus}" unless passed
end
raise 'binary drift' unless Digest::SHA256.file(binary).hexdigest==hash
