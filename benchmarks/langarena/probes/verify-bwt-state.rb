require 'json'
require 'digest'
raise 'usage: OUTPUT' unless ARGV.size == 1
out = File.expand_path(ARGV[0]); manifest = JSON.parse(File.read(out+'/manifest.json'))
raise 'pin' unless manifest['pin'] == '3c9f84ddaeec834c0631747cb789e8cfd602f993'
manifest.fetch('files').each { |p,h| raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest == h }
cases = JSON.parse(File.read(out+'/cases.json')); expected = JSON.parse(File.read(out+'/expected.json'))
raise 'coverage' unless cases.size == 48 && manifest['cases'] == 48 && manifest['states_per_runtime'] == 144
raise 'variants' unless cases.group_by { |c| c['name'] }.transform_values(&:size) == {'Compress::BWTEncode'=>24,'Compress::BWTDecode'=>24}
raise 'byte domain' unless cases.any? { |c| c['data'] == (0..255).to_a }
raise 'state membership' unless expected.map { |r| r.values_at('case','step') } == (0...48).flat_map { |i| (0...3).map { |step| [i,step] } }
commands = manifest.fetch('commands'); raise 'mode coverage' unless commands.map { |c| c['mode'] } == %w[vm interpreter jit]
binary = manifest['files'].keys.first; files = {}
commands.each do |command|
  mode = command['mode']; argv = [binary,'run','--scheduler-timeout-ms','120000']
  argv << "--#{mode}" unless mode == 'vm'; argv << out+'/fixture.kujo'
  raise 'outcome' unless command['argv'] == argv && command['exit'] == 0 && command['signal'].nil?
  raw = File.read(out+'/'+mode+'.out'); actual = raw.lines.map { |line| JSON.parse(line) }
  raise 'full-state mismatch' unless actual == expected
  files[mode+'.out'] = Digest::SHA256.hexdigest(raw); files[mode+'.err'] = Digest::SHA256.file(out+'/'+mode+'.err').hexdigest
  changed = Marshal.load(Marshal.dump(actual)); row = changed.find { |r| !r['encoded'].empty? }; row['encoded'][0] ^= 1
  raise 'altered bytes accepted' if changed == expected
  changed = Marshal.load(Marshal.dump(actual)); row = changed.find { |r| r['encoded'].size > 1 }; row['original'] = (row['original']+1)%row['encoded'].size
  raise 'altered index accepted' if changed == expected
end
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',cases:48,states:432,negative_controls:6,files:files,scope:manifest['scope']}))
puts '432 full states and six unchanged-checksum byte/index corruption controls verified'
