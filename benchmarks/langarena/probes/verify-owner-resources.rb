require 'json'
require 'digest'
require 'open3'
raise 'usage: RESOURCE_DIRECTORY' unless ARGV.size==1
dir=ARGV.first
head,err,status=Open3.capture3('git','rev-parse','HEAD')
raise "source identity #{err}" unless status.success? && File.read(dir+'/source.txt')==head
identity=JSON.parse(File.read(dir+'/executable.json'))
raise 'binary changed' unless Digest::SHA256.file(identity.fetch('path')).hexdigest==identity.fetch('sha256')
cases=JSON.parse(File.read(dir+'/cases.json'))
expected_cases=%w[pooled unpooled].product(%w[same distinct],[32,1024])
raise 'case coverage' unless cases.map{|c|c.values_at('mode','shape','count')}==expected_cases
rows=[]
%w[pooled unpooled].product(%w[same distinct],[32,1024]).each do |mode,shape,count|
  name="#{mode}-#{shape}-#{count}.log"; raw=File.read(dir+'/'+name)
  receipt=cases.find{|c|c.values_at('mode','shape','count')==[mode,shape,count]}
  raise 'process outcome' unless receipt['exit']==0 && receipt['signal'].nil?
  raise 'log identity' unless Digest::SHA256.hexdigest(raw)==receipt['log_sha256']
  raise 'environment' unless receipt['environment']=={'KUJO_OWNER_RESOURCE_MODE'=>mode,'KUJO_OWNER_RESOURCE_SHAPE'=>shape,'KUJO_OWNER_RESOURCE_COUNT'=>count.to_s}
  raise 'invocation' unless receipt['argv']==['timeout','180',identity['path'],'--exact','vm::task_owner_resource_tests::task_owner_resource_diagnostic','--ignored','--nocapture','--test-threads=1']
  raise "test result #{name}" unless raw.scan(/test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out/).size==1
  receipts=raw.scan(/OWNER_RESOURCE (\{[^\n]+\})/).flatten
  raise 'receipt count' unless receipts.size==1
  data=JSON.parse(receipts.first)
  raise 'case identity' unless data.values_at('schema','mode','shape','count')==['kujo.owner-resource/v1',mode,shape,count]
  observations=data.fetch('observations')
  raise 'checkpoint coverage' unless observations.map{|o|o['tasks']}==(32..count).step(32).to_a
  %w[baseline_rss_bytes baseline_executable_mapping_bytes after_drop_rss_bytes after_drop_executable_mapping_bytes].each{|key|raise key unless data[key].is_a?(Integer) && data[key]>0}
  observations.each do |o|
    %w[rss_bytes executable_mapping_bytes].each{|key|raise key unless o[key].is_a?(Integer) && o[key]>0}
    raise 'owner count' unless o['idle_owners'].is_a?(Integer) && (0..2).cover?(o['idle_owners'])
    raise 'unpooled retained owner' if mode=='unpooled' && o['idle_owners']!=0
    raise 'same-code pool not retained' if mode=='pooled' && shape=='same' && o['idle_owners']!=1
  end
  metrics=data.fetch('measurements')
  raise 'metrics contract' unless metrics.values_at('schema','scope','outcome')==['kujo.runtime-measurements/v1','process','success']
  counters=metrics.fetch('counters'); expected=mode=='pooled' && shape=='same' ? 1 : count
  raise 'compile control' unless counters['jit_compile_entries']==expected
  raise 'native participation' unless counters['jit_native_boundary_entries']==count
  rows<<{file:name,sha256:Digest::SHA256.hexdigest(raw),receipt:data}
end
File.write(dir+'/verified.json',JSON.pretty_generate({status:'verified-observations-review-pending',source:head.strip,executable:identity,rows:rows,scope:'Eight isolated diagnostic processes; RSS and executable mappings, not live heap or elapsed speed. Resource tradeoffs require review.'})+"\n")
puts '8 isolated resource observations verified; review retention and memory tradeoffs before acceptance.'
