require 'json'
require 'digest'
raise 'usage: OUTPUT' unless ARGV.size==1
out=File.expand_path(ARGV[0]);manifest=JSON.parse(File.read(out+'/manifest.json'))
raise 'upstream pin' unless manifest['pin']=='3c9f84ddaeec834c0631747cb789e8cfd602f993'
manifest.fetch('files').each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
cases=JSON.parse(File.read(out+'/cases.json'));expected=JSON.parse(File.read(out+'/expected.json'))
raise 'case coverage' unless cases.size==38 && cases.map{|c|c['label']}.uniq.size==38 && manifest['cases']==38
raise 'state coverage' unless expected.map{|r|r.values_at('case','step')}==(0...38).flat_map{|i|(0...3).map{|step|[i,step]}} && manifest['states_per_runtime']==114
commands=manifest.fetch('commands');raise 'mode coverage' unless commands.map{|c|c['mode']}==%w[vm interpreter jit]
files={};binary=manifest['files'].keys.first
commands.each do |c|
 mode=c['mode'];argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
 raise 'command/outcome' unless c['argv']==argv && c['exit']==0 && c['signal'].nil?
 raw=File.read(out+'/'+mode+'.out');actual=raw.lines.map{|line|JSON.parse(line)}
 raise "full-state mismatch #{mode}" unless actual==expected
 files[mode+'.out']=Digest::SHA256.hexdigest(raw);files[mode+'.err']=Digest::SHA256.file(out+'/'+mode+'.err').hexdigest
 # A changed render with its checksum left untouched must not pass full-state checking.
 changed=Marshal.load(Marshal.dump(actual));changed[0]['state']['rendered']+='!'
 raise 'altered render accepted' if changed==expected
end
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',cases:38,states:342,files:files,negative_controls:3,scope:manifest['scope']})+"\n")
puts '342 full-state observations and three unchanged-checksum altered-render controls verified.'
