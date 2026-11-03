require 'json'
require 'digest'
raise 'usage: OUTPUT' unless ARGV.size==1
out=File.expand_path(ARGV[0]);manifest=JSON.parse(File.read(out+'/manifest.json'))
raise 'pin' unless manifest['pin']=='3c9f84ddaeec834c0631747cb789e8cfd602f993'
manifest.fetch('files').each { |p,h| raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h }
cases=JSON.parse(File.read(out+'/cases.json'));expected=JSON.parse(File.read(out+'/expected.json'))
raise 'coverage' unless cases.size==28 && manifest['cases']==28 && manifest['states_per_runtime']==84
raise 'state membership' unless expected.map { |r| r.values_at('case','step') }==(0...28).flat_map { |i| (0...3).map { |step| [i,step] } }
commands=manifest.fetch('commands');raise 'modes' unless commands.map { |c| c['mode'] }==%w[vm interpreter jit]
files={};binary=manifest['files'].keys.first
commands.each do |command|
  mode=command['mode'];argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
  raise 'outcome' unless command['argv']==argv && command['exit']==0 && command['signal'].nil?
  raw=File.read(out+'/'+mode+'.out');actual=raw.lines.map { |line| JSON.parse(line) }
  raise 'full-state mismatch' unless actual==expected
  files[mode+'.out']=Digest::SHA256.hexdigest(raw);files[mode+'.err']=Digest::SHA256.file(out+'/'+mode+'.err').hexdigest
  changed=Marshal.load(Marshal.dump(actual));changed[0]['evaluations'][0]['variables']['fabricated']=1
  raise 'altered variables accepted' if changed==expected
end
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',wrapping_candidate:manifest['wrapping_candidate'],states:252,negative_controls:3,files:files,scope:manifest['scope']}))
puts '252 calculator states and three unchanged-result variable-map controls verified'
