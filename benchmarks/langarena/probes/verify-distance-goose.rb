require 'json'
require 'digest'
state_mode = ARGV.delete('--state')
raise 'usage: GOOSE_EVIDENCE VERIFIED_REFERENCE_EVIDENCE [--state]' unless ARGV.size==2
directory,pairs=ARGV
manifest=JSON.parse(File.read(File.join(directory,'manifest.json')))
manifest.fetch('files').each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
commands=JSON.parse(File.read(File.join(directory,'commands.json')))
raise 'command coverage' unless commands.size==3
commands.each_with_index do |command,i|
  raise 'exit' unless command['exit']==0 && command['signal'].nil?
  %w[stdout stderr].each do |stream|
    suffix=stream=='stdout' ? 'out' : 'err'
    raise 'raw hash' unless Digest::SHA256.file(File.join(directory,"step#{i}.#{suffix}")).hexdigest==command.fetch(stream+'_sha256')
  end
end
expected=File.readlines(File.join(pairs,'rust.out')).map { |l| JSON.parse(l) }
actual=File.readlines(File.join(directory,'step2.out')).map { |l| JSON.parse(l) }
coverage = state_mode ? 64 : 400
raise 'coverage' unless actual.size==coverage && expected.size==coverage
valid=lambda do |rows|
  next rows==expected if state_mode
  rows.each_with_index.all? do |row,i|
    reference=expected[i]
    row.size==5 && row[0]==i && reference[0]==i &&
      (row[1]-reference[1]).abs<=1e-12 && (row[2]-reference[2]).abs<=1e-12 && row[3,2]==reference[3,2]
  end
end
raise 'scores' unless valid.call(actual)
altered=Marshal.load(Marshal.dump(actual))
if state_mode
  altered[0][3] += 1 # Wrong RNG state, same checksum and generated strings.
else
  altered[0][1] += 0.0001
end
raise 'negative accepted' if valid.call(altered)
if state_mode
  raise 'checksum altered' unless altered.map { |r| r[4] }==actual.map { |r| r[4] }
else
  raise 'checksum altered' unless altered[0][3,2]==actual[0][3,2] && (altered[0][1]*1000).to_i==altered[0][3]
end
result={status:'verified',observations:state_mode ? 64 : 800,checksum_preserving_negative_control:'rejected',
        scope:state_mode ? 'Goose complete recorded states; wider coverage remains open' : 'Goose kernel scores only'}
File.write(File.join(directory,'independent-verification.json'),JSON.pretty_generate(result));puts JSON.generate(result)
