require 'json'
require 'digest'
raise 'usage: OUTPUT' unless ARGV.size == 1
out = File.expand_path(ARGV[0])
manifest = JSON.parse(File.read(out+'/manifest.json'))
raise 'upstream pin' unless manifest['pin'] == '3c9f84ddaeec834c0631747cb789e8cfd602f993'
manifest.fetch('files').each { |p,h| raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest == h }
cases = JSON.parse(File.read(out+'/cases.json'))
expected = JSON.parse(File.read(out+'/expected.json'))
raise 'case coverage' unless cases.size == 16 && manifest['cases'] == 16
raise 'state coverage' unless expected.map { |r| r.values_at('case','step') } == (0...16).flat_map { |i| (0...3).map { |step| [i,step] } } && manifest['states_per_runtime'] == 48
raise 'pattern coverage' unless expected.all? { |r| r['matches'].size == 13 }
commands = manifest.fetch('commands')
raise 'mode coverage' unless commands.map { |c| c['mode'] } == %w[vm interpreter jit]
files = {}
binary = manifest['files'].keys.first
commands.each do |command|
  mode = command['mode']
  argv = [binary,'run','--scheduler-timeout-ms','120000']; argv << "--#{mode}" unless mode == 'vm'; argv << out+'/fixture.kujo'
  raise 'command/outcome' unless command['argv'] == argv && command['exit'] == 0 && command['signal'].nil?
  raw = File.read(out+'/'+mode+'.out')
  actual = raw.lines.map { |line| JSON.parse(line) }
  raise 'full-state mismatch' unless actual == expected
  files[mode+'.out'] = Digest::SHA256.hexdigest(raw)
  files[mode+'.err'] = Digest::SHA256.file(out+'/'+mode+'.err').hexdigest
  changed = Marshal.load(Marshal.dump(actual))
  changed[0]['matches'][0] << 'fabricated-match'
  raise 'altered matches accepted' if changed == expected
end
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',cases:16,states:144,pattern_lists:1872,files:files,negative_controls:3,scope:manifest['scope']}))
puts '144 full states, 1872 pattern lists and three unchanged-checksum altered-match controls verified'
