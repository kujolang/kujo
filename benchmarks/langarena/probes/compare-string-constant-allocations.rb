require 'json'
base_hot, base_cold, base_append, cand_hot, cand_cold, cand_append, output = ARGV
raise 'usage: BASE_HOT BASE_COLD BASE_APPEND CAND_HOT CAND_COLD CAND_APPEND NEW_OUTPUT' unless output
raise 'new output required' if File.exist?(output)
read = ->(p) { r=JSON.parse(File.read(p)); raise 'unfinished report' unless r['status']=='measured-review-pending';r }
reports=[base_hot,base_cold,base_append,cand_hot,cand_cold,cand_append].map(&read)
normalize = lambda do |r,kind|
  key_fields = kind=='cold' ? %w[kind width] : %w[mode width reads form]
  rows = kind=='hot' ? r.fetch('allocations') : r.fetch('cases')
  keys = case kind
  when 'cold' then %w[int string].product([0,128,2048])
  else %w[vm jit].product([8,256],[200,2000],%w[literal bound])
  end
  grouped=rows.group_by { |x|x.values_at(*key_fields) }
  raise 'case membership' unless grouped.keys.sort == keys.sort
  grouped.to_h do |key,values|
    raise 'repeat membership' unless values.map{|v|v['round']}.sort == [0,1,2]
    counts=values.map do |v|
      raw=kind=='hot' ? v.fetch('result') : v
      raise 'execution failed' unless raw['exit']==0 && raw['stdout']=="PASS\n"
      counters=raw['stderr'].lines.grep(/^COUNTERS /)
      raise 'counter membership' unless counters.size==1
      entries=JSON.parse(counters.first.delete_prefix('COUNTERS ')).fetch('counters').fetch('jit_native_function_entries')
      raise 'native coverage' unless entries==(key[0]=='jit' ? 2 : 0)
      phases = (kind=='cold' ? [1,2] : [1]).map do |p|
        matches=raw['stderr'].scan(/^PHASE #{p} requests=(\d+) bytes=(\d+)\s*$/)
        raise 'phase membership' unless matches.size==1
        matches.first.map(&:to_i)
      end
      saved=case kind
      when 'hot' then [[v.fetch('requests'),v.fetch('bytes')]]
      when 'cold' then [v.fetch('phases').fetch('1'),v.fetch('phases').fetch('2')]
      else [v.fetch('phase')]
      end
      raise 'raw count mismatch' unless saved==phases
      phases
    end.uniq
    raise 'unstable observations' unless counts.size==1
    [key,counts.first]
  end
end
base=reports.first(3).zip(%w[hot cold append]).map{|r,k|normalize.call(r,k)}
candidate=reports.last(3).zip(%w[hot cold append]).map{|r,k|normalize.call(r,k)}
deltas=base.zip(candidate).zip(%w[hot cold append]).to_h do |(b,c),kind|
  [kind,b.map{|key,phases|{case:key,baseline:phases,candidate:c[key],delta:c[key].zip(phases).map{|x,y|x.zip(y).map{|a,z|a-z}}}}]
end
hot_removed=%w[vm jit].product([8,256],[200,2000]).all? do |mode,width,reads|
  candidate[0].fetch([mode,width,reads,'literal'])==candidate[0].fetch([mode,width,reads,'bound'])
end
no_increase=->(kind,phase){deltas.fetch(kind).all?{|r|r[:delta].fetch(phase).all?{|n|n<=0}}}
result={status:'review-required',gates:{hot_literal_delta_removed:hot_removed,cold_requests_and_bytes_no_increase:no_increase.call('cold',0),warm_requests_and_bytes_no_increase:no_increase.call('cold',1),append_requests_and_bytes_no_increase:no_increase.call('append',0)},deltas:deltas,limits:'Allocation requests/requested bytes only. These gates do not measure retained heap, elapsed performance, all workloads or platforms. Review all tradeoffs; no automatic runtime acceptance.'}
File.write(output,JSON.pretty_generate(result));puts JSON.generate(result[:gates])
