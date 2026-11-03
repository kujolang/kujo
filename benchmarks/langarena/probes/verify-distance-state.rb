require 'json'
require 'digest'
raise 'usage: EVIDENCE_DIRECTORY' unless ARGV.size == 1
directory=ARGV[0]
manifest=JSON.parse(File.read(File.join(directory,'manifest.json')))
manifest.fetch('files').each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
cases=JSON.parse(File.read(File.join(directory,'cases.json')))
raise 'case matrix' unless cases==[[10,5,42,1],[2,500,42,1],[0,1,42,3],[1,1,7,3],[16,32,7,3],[2,4,42,3]]
reference=File.readlines(File.join(directory,'rust.out')).map { |l| JSON.parse(l) }
keys=cases.each_with_index.flat_map { |c,i| 2.times.flat_map { |kind| (0...(3+c[3])).map { |phase| [i,kind,phase] } } }
raise 'Rust coverage/order' unless reference.map { |r| r[0,3] }==keys
cases.each_with_index do |(count,size,seed,iterations),id|
  state=seed
  random=lambda { |limit| state=(3877*state+29573)%139968; (state/139968.0*limit).to_i }
  expected=[]
  count.times do
    a=random.call(size)+4; b=random.call(size)+4
    expected << [a,b].map { |length| length.times.map { (97+random.call(10)).chr }.join }
  end
  reference.select { |r| r[0]==id }.each do |row|
    raise 'pair contents/order' unless row[5]==expected
    raise 'RNG progression' unless row[3]==(row[2]<2 ? state : 42)
    raise 'initial result' if row[2]==0 && row[4]!=0
  end
end
runs=JSON.parse(File.read(File.join(directory,'runs.json')))
raise 'modes' unless runs.map { |r| r['runtime'] }.sort==%w[interpreter jit vm]
runs.each do |run|
  raise 'exit' unless run['exit']==0 && run['signal'].nil?
  %w[stdout stderr].each do |stream|
    suffix=stream=='stdout' ? 'out' : 'err'
    raise 'raw hash' unless Digest::SHA256.file(File.join(directory,run['runtime']+'.'+suffix)).hexdigest==run.fetch(stream+'_sha256')
  end
  rows=File.readlines(File.join(directory,run['runtime']+'.out')).map { |l| JSON.parse(l) }
  raise 'complete state mismatch' unless rows==reference
  altered=Marshal.load(Marshal.dump(rows))
  altered[0][5][0][0]='incorrect preparation'
  raise 'negative accepted' if altered==reference
  raise 'checksum changed' unless altered.map { |r| r[4] }==rows.map { |r| r[4] }
end
result={status:'verified',complete_states:192,checksum_preserving_pair_negative_controls:3,
        scope:'Complete recorded states; production, work counts, executed Goose remain open'}
File.write(File.join(directory,'independent-verification.json'),JSON.pretty_generate(result))
puts JSON.generate(result)
