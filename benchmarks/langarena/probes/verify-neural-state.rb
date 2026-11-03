require 'json'; require 'digest'
raise 'usage: OUTPUT' unless ARGV.size==1
out=File.expand_path(ARGV[0]);report=JSON.parse(File.read(out+'/verification.json'))
raise 'pin' unless report['pin']=='3c9f84ddaeec834c0631747cb789e8cfd602f993'
report['files'].each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
expected=File.readlines(out+'/rust.out').map{|l|JSON.parse(l)}
order=[42,1,139967].flat_map{|seed| [[seed,'prepare',0]]+(0...128).map{|i|[seed,'train',i]}+(0...4).map{|i|[seed,'forward',i]}}
raise 'oracle membership' unless expected.map{|r|r.values_at('seed','phase','step')}==order
raise 'topology coverage' unless expected.all?{|r|r['neurons'].size==13 && r['synapses'].size==30 && r['neurons'].all?{|n|n.size==6} && r['synapses'].all?{|s|s.size==4}}
# Exact observed equality is stronger than the collector's portable 1e-12 policy.
raise 'runtime coverage' unless report['rows'].map{|r|r['runtime']}==%w[vm interpreter jit]
controls=[]
report['rows'].each do |row|
 mode=row['runtime'];raise 'outcome' unless row['exit']==0 && row['states']==399
 actual=File.readlines(out+'/'+mode+'.out').map{|l|JSON.parse(l)}
 raise 'full-state difference' unless actual==expected
 %w[weight link output previous].each do |control|
  changed=Marshal.load(Marshal.dump(actual))
  case control
  when 'weight' then changed[1]['synapses'][0][2]+=0.001
  when 'link' then changed[1]['neurons'][2][4][0]+=1
  when 'output' then changed[1]['neurons'][12][2]+=0.001
  when 'previous' then changed[1]['synapses'][20][3]+=0.001
  end
  raise 'mutant accepted' if changed==expected
  controls<<[mode,control]
 end
end
raise 'collector count' unless report['states']==1197 && report['float_checks']==134064 && report['max_absolute_error']==0.0
result={status:'verified-exact',states:1197,float_values:134064,negative_controls:controls,scope:report['scope']}
File.write(out+'/independent-review.json',JSON.pretty_generate(result));puts JSON.generate(result)
