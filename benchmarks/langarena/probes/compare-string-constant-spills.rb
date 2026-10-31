require 'json';require 'digest'
base_path,candidate_path,output=ARGV
raise 'usage: BASE_REPORT CANDIDATE_REPORT NEW_OUTPUT' unless output
raise 'new output required' if File.exist?(output)
read= lambda do |path|
  r=JSON.parse(File.read(path));raise 'incomplete' unless r['status']=='measured-review-pending'
  groups=r.fetch('cases').group_by{|x|x.values_at('mode','keys','width','reads')}
  raise 'case membership' unless groups.keys.sort==%w[vm jit].product([1,4,16],[8,256],[200,2000]).sort
  groups.each do |key,rows|
    raise 'repeat membership' unless rows.map{|x|x['round']}.sort==[0,1,2]
    rows.each do |x|
      raise 'output' unless x['exit']==0 && x['stdout']=="PASS\n"
      counters=x['stderr'].lines.grep(/^COUNTERS /);raise 'counters' unless counters.size==1
      entries=JSON.parse(counters.first.delete_prefix('COUNTERS ')).fetch('counters').fetch('jit_native_function_entries')
      raise 'native coverage' unless entries==x['native_entries'] && entries==(key[0]=='jit' ? 2 : 0)
      [1,2].each do |p|
        raw=x['stderr'].scan(/^PHASE #{p} requests=(\d+) bytes=(\d+)\s*$/)
        raise 'phase counts' unless raw.size==1 && raw.first.map(&:to_i)==x['phases'].fetch(p.to_s)
      end
    end
    raise 'unstable warm counts' unless rows.map{|x|x['phases']['2']}.uniq.size==1
    raise 'unstable cold VM counts' if key[0]=='vm' && rows.map{|x|x['phases']['1']}.uniq.size!=1
  end
  groups
end
base=read.call(base_path);candidate=read.call(candidate_path)
rows=base.map do |key,values|
  other=candidate.fetch(key)
  raise 'workload mismatch' unless values.map{|x|x['source']}.uniq==other.map{|x|x['source']}.uniq
  warm=values.first['phases']['2'];new_warm=other.first['phases']['2']
  {case:key,warm_baseline:warm,warm_candidate:new_warm,warm_delta:new_warm.zip(warm).map{|a,b|a-b},cold_baseline:values.map{|x|x['phases']['1']},cold_candidate:other.map{|x|x['phases']['1']}}
end
slopes=[1,4,16].product([8,256]).map do |keys,width|
  low=rows.find{|x|x[:case]==['vm',keys,width,200]}[:warm_delta]
  high=rows.find{|x|x[:case]==['vm',keys,width,2000]}[:warm_delta]
  actual=high.zip(low).map{|a,b|a-b};expected=[-2*keys*1800,-(width+40)*keys*1800]
  {keys:keys,width:width,actual:actual,expected:expected,pass:actual==expected}
end
result={status:'review-required',reports:[base_path,candidate_path].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},warm_literal_allocation_slopes_removed:slopes.all?{|x|x[:pass]},warm_requests_and_bytes_no_increase:rows.all?{|x|x[:warm_delta].all?{|n|n<=0}},slopes:slopes,groups:rows,limits:'Cold JIT includes compilation and is not a deterministic cache-only measurement. Allocation results do not establish elapsed speed or retained memory.'}
File.write(output,JSON.pretty_generate(result));puts JSON.generate(result.select{|k,v|[:warm_literal_allocation_slopes_removed,:warm_requests_and_bytes_no_increase].include?(k)})
